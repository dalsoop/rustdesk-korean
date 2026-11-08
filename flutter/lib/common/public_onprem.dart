// Korean fork, RustDesk Public flavor: on-prem (Tailscale) recents and favorites.
// Kept apart from RustDesk's own peer lists so they never mix with internet IDs.

import 'dart:convert';

import 'tailscale.dart';

const String kOptionPublicOnPremRecords = 'public-onprem-records';

class OnPremRecord {
  final String profileId;
  final String tailnet;
  final String deviceName;
  final String lastIp;
  final bool favorite;
  // Milliseconds since epoch; 0 when the device was only favorited.
  final int lastConnected;

  const OnPremRecord({
    required this.profileId,
    required this.tailnet,
    required this.deviceName,
    required this.lastIp,
    required this.favorite,
    required this.lastConnected,
  });

  OnPremRecord copyWith(
      {String? profileId, String? lastIp, bool? favorite, int? lastConnected}) {
    return OnPremRecord(
      profileId: profileId ?? this.profileId,
      tailnet: tailnet,
      deviceName: deviceName,
      lastIp: lastIp ?? this.lastIp,
      favorite: favorite ?? this.favorite,
      lastConnected: lastConnected ?? this.lastConnected,
    );
  }

  Map<String, dynamic> toJson() => {
        'profile_id': profileId,
        'tailnet': tailnet,
        'device_name': deviceName,
        'last_ip': lastIp,
        'favorite': favorite,
        'last_connected': lastConnected,
      };

  static OnPremRecord? fromJson(Object? json) {
    if (json is! Map<String, dynamic>) return null;
    final tailnet = json['tailnet'];
    final name = json['device_name'];
    if (tailnet is! String || name is! String || name.isEmpty) return null;
    final lastConnected = json['last_connected'];
    return OnPremRecord(
      profileId: (json['profile_id'] as String?) ?? '',
      tailnet: tailnet,
      deviceName: name,
      lastIp: (json['last_ip'] as String?) ?? '',
      favorite: json['favorite'] == true,
      lastConnected: lastConnected is int ? lastConnected : 0,
    );
  }

  bool matches(String tailnet, String deviceName) =>
      this.tailnet == tailnet && this.deviceName == deviceName;
}

List<OnPremRecord> decodeOnPremRecords(String text) {
  if (text.isEmpty) return [];
  try {
    final decoded = jsonDecode(text);
    if (decoded is! List) return [];
    return decoded.map(OnPremRecord.fromJson).whereType<OnPremRecord>().toList();
  } catch (_) {
    return [];
  }
}

String encodeOnPremRecords(List<OnPremRecord> records) =>
    jsonEncode(records.map((r) => r.toJson()).toList());

/// The records of one tailnet: favorites first, then the most recent.
List<OnPremRecord> onPremRecordsFor(List<OnPremRecord> all, String tailnet) {
  final list = all.where((r) => r.tailnet == tailnet).toList();
  list.sort((a, b) {
    if (a.favorite != b.favorite) return a.favorite ? -1 : 1;
    return b.lastConnected.compareTo(a.lastConnected);
  });
  return list;
}

List<OnPremRecord> recordOnPremConnection(List<OnPremRecord> all,
    TailscaleProfile profile, TailscaleDevice device, int nowMs) {
  final ip = device.ipv4 ?? '';
  var found = false;
  final next = all.map((r) {
    if (!r.matches(profile.tailnet, device.name)) return r;
    found = true;
    return r.copyWith(profileId: profile.id, lastIp: ip, lastConnected: nowMs);
  }).toList();
  if (!found) {
    next.add(OnPremRecord(
      profileId: profile.id,
      tailnet: profile.tailnet,
      deviceName: device.name,
      lastIp: ip,
      favorite: false,
      lastConnected: nowMs,
    ));
  }
  return next;
}

/// A record that is neither a favorite nor ever connected is dropped.
List<OnPremRecord> toggleOnPremFavorite(List<OnPremRecord> all,
    TailscaleProfile profile, String deviceName, String? ip) {
  final next = <OnPremRecord>[];
  var found = false;
  for (final r in all) {
    if (!r.matches(profile.tailnet, deviceName)) {
      next.add(r);
      continue;
    }
    found = true;
    final toggled = r.copyWith(profileId: profile.id, favorite: !r.favorite);
    if (toggled.favorite || toggled.lastConnected > 0) next.add(toggled);
  }
  if (!found) {
    next.add(OnPremRecord(
      profileId: profile.id,
      tailnet: profile.tailnet,
      deviceName: deviceName,
      lastIp: ip ?? '',
      favorite: true,
      lastConnected: 0,
    ));
  }
  return next;
}

class OnPremEntry {
  /// The device as Tailscale lists it now; null when its name is gone.
  final TailscaleDevice? device;
  final OnPremRecord? record;

  const OnPremEntry({required this.device, required this.record});

  String get name => device?.name ?? record?.deviceName ?? '';
  bool get missing => device == null;
  bool get isFavorite => record?.favorite == true;

  /// Always the device's current Tailscale IPv4, never the stored one.
  String? get connectIp => device?.ipv4;
}

/// Remembered devices first (in [records] order), then the rest of [devices].
List<OnPremEntry> buildOnPremEntries(
    List<TailscaleDevice> devices, List<OnPremRecord> records) {
  final byName = <String, TailscaleDevice>{};
  for (final d in devices) {
    byName.putIfAbsent(d.name, () => d);
  }
  final used = <String>{};
  final entries = <OnPremEntry>[];
  for (final r in records) {
    if (!used.add(r.deviceName)) continue;
    entries.add(OnPremEntry(device: byName[r.deviceName], record: r));
  }
  for (final d in devices) {
    if (used.contains(d.name)) continue;
    entries.add(OnPremEntry(device: d, record: null));
  }
  return entries;
}
