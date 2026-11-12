import 'dart:async';
import 'dart:convert';
import 'dart:io';

const int kRustDeskDirectPort = 21118;
const Duration kRustDeskPortCheckTimeout = Duration(milliseconds: 1500);
const Duration kTailscaleCommandTimeout = Duration(seconds: 5);
const Duration kTailscaleSwitchTimeout = Duration(seconds: 20);

const String _kTailscaleAppBinary =
    '/Applications/Tailscale.app/Contents/MacOS/Tailscale';
// GUI apps get a minimal PATH, so a Homebrew CLI would not be found through it.
const List<String> _kExtraBinaryDirs = ['/opt/homebrew/bin', '/usr/local/bin'];

enum TailscaleState { notFound, stopped, needsLogin, needsApproval, running }

enum RustDeskReachability { checking, reachable, unreachable }

class TailscaleProfile {
  final String id;
  final String tailnet;
  final String account;
  final bool active;

  const TailscaleProfile({
    required this.id,
    required this.tailnet,
    required this.account,
    required this.active,
  });
}

class TailscaleDevice {
  final String id;
  final String name;
  final String os;
  final bool online;
  final String? ipv4;
  final String dnsName;
  final List<String> ips;
  // Non-empty when traffic goes straight to the peer at this address.
  final String curAddr;
  // DERP region used when there is no direct path.
  final String relay;
  final DateTime? lastSeen;
  final String owner;
  final List<String> tags;

  const TailscaleDevice({
    required this.id,
    required this.name,
    required this.os,
    required this.online,
    required this.ipv4,
    this.dnsName = '',
    this.ips = const [],
    this.curAddr = '',
    this.relay = '',
    this.lastSeen,
    this.owner = '',
    this.tags = const [],
  });
}

/// What a peer's RustDesk LAN discovery listener said about itself.
class RustDeskPeerInfo {
  final String id;
  final String hostname;
  final String platform;

  const RustDeskPeerInfo(
      {required this.id, required this.hostname, required this.platform});
}

/// Parses the JSON from `mainPublicProbeRustdesk`; empty or bad text means
/// nothing answered.
RustDeskPeerInfo? parseRustDeskProbe(String output) {
  if (output.isEmpty) return null;
  try {
    final decoded = jsonDecode(output);
    if (decoded is! Map<String, dynamic>) return null;
    return RustDeskPeerInfo(
      id: (decoded['id'] as String?) ?? '',
      hostname: (decoded['hostname'] as String?) ?? '',
      platform: (decoded['platform'] as String?) ?? '',
    );
  } catch (_) {
    return null;
  }
}

class TailscaleStatus {
  final TailscaleState state;
  final List<TailscaleDevice> devices;

  const TailscaleStatus({required this.state, required this.devices});

  static const notFound =
      TailscaleStatus(state: TailscaleState.notFound, devices: []);
}

class TailscaleSnapshot {
  final TailscaleState state;
  final List<TailscaleProfile> profiles;
  final List<TailscaleDevice> devices;

  const TailscaleSnapshot({
    required this.state,
    required this.profiles,
    required this.devices,
  });

  TailscaleProfile? get activeProfile {
    for (final p in profiles) {
      if (p.active) return p;
    }
    return null;
  }
}

/// Parses the table printed by `tailscale switch --list`; `*` after the
/// account marks the active profile.
List<TailscaleProfile> parseTailscaleProfiles(String output) {
  final profiles = <TailscaleProfile>[];
  for (final raw in const LineSplitter().convert(output)) {
    final line = raw.trim();
    if (line.isEmpty) continue;
    final cols = line.split(RegExp(r'\s+'));
    if (cols.length < 3) continue;
    if (cols.first == 'ID' && cols[1] == 'Tailnet') continue;
    var account = cols.last;
    final active = account.endsWith('*');
    if (active) account = account.substring(0, account.length - 1);
    profiles.add(TailscaleProfile(
      id: cols.first,
      tailnet: cols.sublist(1, cols.length - 1).join(' '),
      account: account,
      active: active,
    ));
  }
  return profiles;
}

/// Parses `tailscale status --json`. Returns [TailscaleStatus.notFound] when
/// the text is not a status object.
TailscaleStatus parseTailscaleStatusJson(String output) {
  final Object? decoded;
  try {
    decoded = jsonDecode(output);
  } catch (_) {
    return TailscaleStatus.notFound;
  }
  if (decoded is! Map<String, dynamic>) return TailscaleStatus.notFound;

  final state = switch (decoded['BackendState']) {
    'Running' => TailscaleState.running,
    'NeedsLogin' => TailscaleState.needsLogin,
    'NeedsMachineAuth' => TailscaleState.needsApproval,
    _ => TailscaleState.stopped,
  };

  final self = decoded['Self'];
  final selfId = self is Map<String, dynamic> ? self['ID'] : null;
  final users = decoded['User'];

  final devices = <TailscaleDevice>[];
  final peers = decoded['Peer'];
  if (peers is Map<String, dynamic>) {
    for (final entry in peers.entries) {
      final peer = entry.value;
      if (peer is! Map<String, dynamic>) continue;
      final id = (peer['ID'] as String?) ?? entry.key;
      if (selfId != null && id == selfId) continue;
      devices.add(TailscaleDevice(
        id: id,
        name: _deviceName(peer),
        os: (peer['OS'] as String?) ?? '',
        online: peer['Online'] == true,
        ipv4: _firstIpv4(peer['TailscaleIPs']),
        dnsName: _dnsName(peer['DNSName']),
        ips: _stringList(peer['TailscaleIPs']),
        curAddr: (peer['CurAddr'] as String?) ?? '',
        relay: (peer['Relay'] as String?) ?? '',
        lastSeen: _lastSeen(peer['LastSeen']),
        owner: _ownerLogin(users, peer['UserID']),
        tags: _stringList(peer['Tags']),
      ));
    }
  }
  devices.sort((a, b) => a.name.toLowerCase().compareTo(b.name.toLowerCase()));
  return TailscaleStatus(state: state, devices: devices);
}

String _deviceName(Map<String, dynamic> peer) {
  final dns = peer['DNSName'];
  if (dns is String) {
    final label = dns.split('.').first;
    if (label.isNotEmpty) return label;
  }
  return (peer['HostName'] as String?) ?? '';
}

String? _firstIpv4(Object? ips) {
  if (ips is! List) return null;
  for (final ip in ips) {
    if (ip is! String) continue;
    if (InternetAddress.tryParse(ip)?.type == InternetAddressType.IPv4) {
      return ip;
    }
  }
  return null;
}

String _dnsName(Object? dns) {
  if (dns is! String) return '';
  return dns.endsWith('.') ? dns.substring(0, dns.length - 1) : dns;
}

List<String> _stringList(Object? value) {
  if (value is! List) return const [];
  return value.whereType<String>().toList();
}

// Tailscale writes the zero time (year 1) for a peer it has not seen yet.
DateTime? _lastSeen(Object? value) {
  if (value is! String) return null;
  final t = DateTime.tryParse(value);
  if (t == null || t.year <= 1) return null;
  return t;
}

String _ownerLogin(Object? users, Object? userId) {
  if (users is! Map<String, dynamic> || userId == null) return '';
  final user = users['$userId'];
  if (user is! Map<String, dynamic>) return '';
  return (user['LoginName'] as String?) ?? '';
}

class _CommandResult {
  final int exitCode;
  final String stdout;

  const _CommandResult(this.exitCode, this.stdout);
}

class Tailscale {
  static String? findBinary() {
    if (File(_kTailscaleAppBinary).existsSync()) return _kTailscaleAppBinary;
    final pathDirs = (Platform.environment['PATH'] ?? '')
        .split(':')
        .where((d) => d.isNotEmpty);
    for (final dir in [...pathDirs, ..._kExtraBinaryDirs]) {
      final candidate = '$dir/tailscale';
      if (File(candidate).existsSync()) return candidate;
    }
    return null;
  }

  static Future<_CommandResult?> _run(List<String> args, Duration timeout) async {
    final binary = findBinary();
    if (binary == null) return null;
    final Process process;
    try {
      process = await Process.start(binary, args);
    } catch (_) {
      return null;
    }
    final stdout = process.stdout.transform(utf8.decoder).join();
    process.stderr.drain<void>();
    try {
      final code = await process.exitCode.timeout(timeout);
      return _CommandResult(code, await stdout.timeout(timeout));
    } on TimeoutException {
      process.kill();
      return null;
    }
  }

  static Future<List<TailscaleProfile>> listProfiles() async {
    final result = await _run(['switch', '--list'], kTailscaleCommandTimeout);
    if (result == null || result.exitCode != 0) return [];
    return parseTailscaleProfiles(result.stdout);
  }

  /// A binary that runs but prints no status object counts as stopped;
  /// one that is missing or does not answer in time counts as not found.
  static Future<TailscaleStatus> readStatus() async {
    final result = await _run(['status', '--json'], kTailscaleCommandTimeout);
    if (result == null) return TailscaleStatus.notFound;
    final status = parseTailscaleStatusJson(result.stdout);
    if (status.state == TailscaleState.notFound) {
      return const TailscaleStatus(state: TailscaleState.stopped, devices: []);
    }
    return status;
  }

  static Future<TailscaleSnapshot> read() async {
    final results = await Future.wait([readStatus(), listProfiles()]);
    final status = results[0] as TailscaleStatus;
    return TailscaleSnapshot(
      state: status.state,
      profiles: results[1] as List<TailscaleProfile>,
      devices: status.devices,
    );
  }

  static Future<bool> switchProfile(String profileId) async {
    final result = await _run(['switch', profileId], kTailscaleSwitchTimeout);
    return result != null && result.exitCode == 0;
  }

  /// Brings up the Tailscale app (standalone or App Store build alike).
  static Future<bool> openApp() async {
    try {
      final result = await Process.run('/usr/bin/open', ['-a', 'Tailscale']);
      return result.exitCode == 0;
    } catch (_) {
      return false;
    }
  }

  static Future<RustDeskReachability> checkRustDesk(
    String ip, {
    Duration timeout = kRustDeskPortCheckTimeout,
  }) async {
    try {
      final socket =
          await Socket.connect(ip, kRustDeskDirectPort, timeout: timeout);
      socket.destroy();
      return RustDeskReachability.reachable;
    } catch (_) {
      return RustDeskReachability.unreachable;
    }
  }
}
