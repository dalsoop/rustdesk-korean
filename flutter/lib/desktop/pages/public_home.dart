// Korean fork, RustDesk Public flavor: the right pane of the main window.
// Tabs 온프레미스 (this Mac's Tailscale tailnets) | 인터넷 (stock ConnectionPage).

import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_hbb/common.dart';
import 'package:flutter_hbb/common/public_onprem.dart';
import 'package:flutter_hbb/common/tailscale.dart';
import 'package:flutter_hbb/consts.dart';
import 'package:flutter_hbb/desktop/pages/connection_page.dart';
import 'package:flutter_hbb/desktop/pages/public_onboarding.dart';
import 'package:flutter_hbb/models/platform_model.dart';
import 'package:get/get.dart';

const double _kCardWidth = 220;
const double _kStatusBarHeight = 42;
// While Tailscale is not usable yet, so the guide moves on by itself once the
// user has installed, turned on or signed in to it.
const Duration _kSetupPollInterval = Duration(seconds: 5);

enum _RustDeskState { checking, reachable, directOff, missing, unreachable }

class PublicHomePage extends StatefulWidget {
  const PublicHomePage({Key? key}) : super(key: key);

  @override
  State<PublicHomePage> createState() => _PublicHomePageState();
}

class _PublicHomePageState extends State<PublicHomePage> {
  static const int _onPremTab = 0;
  static const int _internetTab = 1;

  // Opens on the tab chosen in the welcome guide (온프레미스 when none was
  // chosen); later tab clicks are deliberately not remembered.
  int _tab = publicStartsOnInternet ? _internetTab : _onPremTab;
  TailscaleSnapshot? _snapshot;
  bool _loading = false;
  bool _switching = false;
  bool _showOthers = false;
  Timer? _setupPoll;
  String? _selectedProfileId;
  int _readSeq = 0;
  final Map<String, RustDeskReachability> _reach = {};
  // A key without a value: the discovery probe got no answer.
  final Map<String, RustDeskPeerInfo?> _probe = {};
  List<OnPremRecord> _records = [];

  @override
  void initState() {
    super.initState();
    _records = decodeOnPremRecords(
        bind.getLocalFlutterOption(k: kOptionPublicOnPremRecords));
    _refresh();
    _setupPoll = Timer.periodic(_kSetupPollInterval, (_) {
      final state = _snapshot?.state;
      if (_tab == _onPremTab &&
          !_loading &&
          !_switching &&
          state != null &&
          state != TailscaleState.running) {
        _refresh();
      }
    });
    if (!publicWelcomeDone) {
      WidgetsBinding.instance
          .addPostFrameCallback((_) => showPublicWelcome(_onWelcomeChoice));
    }
  }

  @override
  void dispose() {
    _setupPoll?.cancel();
    super.dispose();
  }

  void _onWelcomeChoice(bool internet) {
    if (mounted) _selectTab(internet ? _internetTab : _onPremTab);
  }

  void _saveRecords(List<OnPremRecord> records) {
    setState(() => _records = records);
    bind.setLocalFlutterOption(
        k: kOptionPublicOnPremRecords, v: encodeOnPremRecords(records));
  }

  Future<void> _refresh() async {
    final seq = ++_readSeq;
    setState(() => _loading = true);
    final snapshot = await Tailscale.read();
    if (!mounted || seq != _readSeq) return;
    final ids = snapshot.profiles.map((p) => p.id).toSet();
    setState(() {
      _snapshot = snapshot;
      _loading = false;
      if (!ids.contains(_selectedProfileId)) {
        _selectedProfileId = snapshot.activeProfile?.id ??
            (snapshot.profiles.isEmpty ? null : snapshot.profiles.first.id);
      }
      _reach.clear();
      _probe.clear();
    });
    if (snapshot.state != TailscaleState.running) return;
    for (final d in snapshot.devices) {
      final ip = d.ipv4;
      if (ip == null) {
        _reach[d.id] = RustDeskReachability.unreachable;
        continue;
      }
      _reach[d.id] = RustDeskReachability.checking;
      Tailscale.checkRustDesk(ip).then((result) {
        if (!mounted || seq != _readSeq) return;
        setState(() => _reach[d.id] = result);
      });
      // An offline peer cannot answer; skipping it keeps the bridge's
      // worker threads free for the peers that can.
      if (!d.online) continue;
      bind.mainPublicProbeRustdesk(ip: ip).then((out) {
        if (!mounted || seq != _readSeq) return;
        setState(() => _probe[d.id] = parseRustDeskProbe(out));
      });
    }
    setState(() {});
  }

  void _selectTab(int tab) {
    if (_tab == tab) return;
    setState(() => _tab = tab);
    if (tab == _onPremTab) _refresh();
  }

  TailscaleProfile? get _selectedProfile {
    for (final p in _snapshot?.profiles ?? const <TailscaleProfile>[]) {
      if (p.id == _selectedProfileId) return p;
    }
    return null;
  }

  // Without `switch --list` support there is no profile; the running tailnet
  // is still usable, its records keyed by an empty tailnet name.
  TailscaleProfile get _recordProfile =>
      _selectedProfile ??
      const TailscaleProfile(id: '', tailnet: '', account: '', active: true);

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        _buildTabBar(context),
        const Divider(height: 1),
        Expanded(
          child: IndexedStack(
            index: _tab,
            children: [
              _buildOnPrem(context),
              Column(
                children: [
                  const PublicInternetLoginBanner(),
                  Expanded(child: ConnectionPage()),
                ],
              ),
            ],
          ),
        ),
        const Divider(height: 1),
        Offstage(
          offstage: _tab != _onPremTab,
          child: _buildOnPremStatus(),
        ),
        // Kept mounted on both tabs: it also polls the service state for
        // the rest of the app.
        Offstage(
          offstage: _tab != _internetTab,
          child: Row(
            children: [
              Expanded(child: OnlineStatusWidget()),
              Obx(() => Text(
                    gFFI.userModel.userName.value.isEmpty
                        ? translate('Not logged in')
                        : translate(
                            'Logged in: {${gFFI.userModel.userName.value}}'),
                    style: const TextStyle(fontSize: 14),
                  )).marginOnly(right: 14),
            ],
          ),
        ),
      ],
    );
  }

  Widget _buildTabBar(BuildContext context) {
    Widget tab(int index, String label) {
      final selected = _tab == index;
      return InkWell(
        onTap: () => _selectTab(index),
        child: Container(
          padding: const EdgeInsets.symmetric(horizontal: 18, vertical: 10),
          decoration: BoxDecoration(
            border: Border(
              bottom: BorderSide(
                color: selected ? MyTheme.accent : Colors.transparent,
                width: 2,
              ),
            ),
          ),
          child: Text(
            translate(label),
            style: TextStyle(
              fontSize: 15,
              fontWeight: selected ? FontWeight.bold : FontWeight.normal,
              color: selected
                  ? Theme.of(context).textTheme.titleLarge?.color
                  : Theme.of(context).hintColor,
            ),
          ),
        ),
      );
    }

    return Row(
      children: [
        tab(_onPremTab, 'On-premises'),
        tab(_internetTab, 'Internet'),
        const Spacer(),
        IconButton(
          tooltip: translate('Show the welcome guide'),
          icon: const Icon(Icons.help_outline, size: 20),
          onPressed: () => showPublicWelcome(_onWelcomeChoice),
        ),
      ],
    ).marginOnly(left: 12, right: 8, top: 8);
  }

  Widget _buildOnPremStatus() {
    final snapshot = _snapshot;
    final String text;
    if (snapshot == null) {
      text = '';
    } else if (snapshot.state == TailscaleState.notFound) {
      text = translate('Tailscale: not found');
    } else if (snapshot.state == TailscaleState.stopped) {
      text = translate('Tailscale is turned off');
    } else if (snapshot.state == TailscaleState.needsLogin) {
      text = translate('Tailscale: sign-in needed');
    } else if (snapshot.state == TailscaleState.needsApproval) {
      text = translate('Tailscale: waiting for approval');
    } else {
      text = translate(
          'Tailscale: {${snapshot.activeProfile?.tailnet ?? ''}} connected');
    }
    return SizedBox(
      height: _kStatusBarHeight,
      child: Align(
        alignment: Alignment.centerLeft,
        child: Text(text, style: const TextStyle(fontSize: 14))
            .marginOnly(left: 14),
      ),
    );
  }

  Widget _buildOnPrem(BuildContext context) {
    final snapshot = _snapshot;
    if (snapshot == null) {
      return const Center(child: CircularProgressIndicator(strokeWidth: 2));
    }
    if (snapshot.state == TailscaleState.notFound) {
      return _buildGuide(snapshot.state);
    }
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            if (snapshot.profiles.isNotEmpty) _buildProfilePicker(snapshot),
            const Spacer(),
            if (_loading)
              const SizedBox(
                width: 16,
                height: 16,
                child: CircularProgressIndicator(strokeWidth: 2),
              ).marginOnly(right: 8),
            IconButton(
              tooltip: translate('Refresh'),
              icon: const Icon(Icons.refresh, size: 20),
              onPressed: _loading || _switching ? null : _refresh,
            ),
          ],
        ).marginOnly(left: 12, right: 12, top: 12),
        Expanded(child: _buildDeviceArea(context, snapshot)),
      ],
    );
  }

  Widget _buildProfilePicker(TailscaleSnapshot snapshot) {
    return DropdownButton<String>(
      value: _selectedProfileId,
      underline: const SizedBox.shrink(),
      items: snapshot.profiles
          .map((p) => DropdownMenuItem<String>(
                value: p.id,
                child: Text(p.tailnet == p.account
                    ? p.tailnet
                    : '${p.tailnet} (${p.account})'),
              ))
          .toList(),
      onChanged: _switching
          ? null
          : (id) => setState(() => _selectedProfileId = id),
    );
  }

  Widget _buildDeviceArea(BuildContext context, TailscaleSnapshot snapshot) {
    if (snapshot.state != TailscaleState.running) {
      return _buildGuide(snapshot.state);
    }
    final selected = _selectedProfile;
    if (selected != null && !selected.active) {
      return _buildSwitchArea(context, snapshot, selected);
    }
    final entries = buildOnPremEntries(
        snapshot.devices, onPremRecordsFor(_records, _recordProfile.tailnet));
    if (entries.isEmpty) {
      return _buildGuide(TailscaleState.running);
    }
    // Remembered devices and the ones that can (or may) take a connection
    // come first; the rest would bury them on a tailnet full of servers.
    final ready = <OnPremEntry>[];
    final others = <OnPremEntry>[];
    for (final e in entries) {
      (_isReady(e) ? ready : others).add(e);
    }
    final hint = TextStyle(fontSize: 13, color: Theme.of(context).hintColor);
    return SingleChildScrollView(
      padding: const EdgeInsets.all(12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          if (ready.isEmpty) Text(translate('public_no_ready_tip'), style: hint),
          if (ready.isNotEmpty)
            Wrap(
              spacing: 12,
              runSpacing: 12,
              children: ready.map((e) => _buildCard(context, e)).toList(),
            ),
          if (others.isNotEmpty)
            InkWell(
              onTap: () => setState(() => _showOthers = !_showOthers),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Icon(_showOthers ? Icons.expand_less : Icons.expand_more,
                      size: 18),
                  const SizedBox(width: 4),
                  Text(translate('Other devices ({${others.length}})'),
                      style: hint),
                ],
              ),
            ).marginOnly(top: 16, bottom: 8),
          if (others.isNotEmpty && _showOthers)
            Wrap(
              spacing: 12,
              runSpacing: 12,
              children: others.map((e) => _buildCard(context, e)).toList(),
            ),
        ],
      ),
    );
  }

  bool _isReady(OnPremEntry entry) {
    if (entry.record != null) return true;
    final device = entry.device;
    if (device == null) return false;
    return switch (_rustDeskState(device)) {
      _RustDeskState.reachable ||
      _RustDeskState.directOff ||
      _RustDeskState.checking =>
        true,
      _RustDeskState.missing || _RustDeskState.unreachable => false,
    };
  }

  Widget _buildGuide(TailscaleState state) {
    return PublicTailscaleGuide(
      state: state,
      onCheckAgain: _loading || _switching ? null : _refresh,
      onUseInternet: () => _selectTab(_internetTab),
    );
  }

  Widget _buildSwitchArea(BuildContext context, TailscaleSnapshot snapshot,
      TailscaleProfile target) {
    final active = snapshot.activeProfile?.tailnet ?? '';
    final records = onPremRecordsFor(_records, target.tailnet);
    return SingleChildScrollView(
      padding: const EdgeInsets.all(12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          ElevatedButton(
            onPressed:
                _switching ? null : () => _confirmSwitch(target, active),
            child: Text(translate('Switch to {${target.tailnet}}')),
          ),
          const SizedBox(height: 10),
          Text(
            translate(
                'Switching disconnects the current network ({$active}). SSH, remote terminals and this RustDesk connection will drop.'),
            style: const TextStyle(color: kColorWarn),
          ),
          if (records.isNotEmpty) const SizedBox(height: 16),
          if (records.isNotEmpty)
            Wrap(
              spacing: 12,
              runSpacing: 12,
              children: records
                  .map((r) => _buildCard(
                        context,
                        OnPremEntry(device: null, record: r),
                        inactiveProfile: true,
                      ))
                  .toList(),
            ),
        ],
      ),
    );
  }

  void _confirmSwitch(TailscaleProfile target, String active) {
    gFFI.dialogManager.show((setState, close, context) {
      submit() {
        close();
        _switchTo(target);
      }

      return CustomAlertDialog(
        title: Text(translate('Switch to {${target.tailnet}}')),
        content: Text(translate(
            'Switching disconnects the current network ({$active}). SSH, remote terminals and this RustDesk connection will drop.')),
        actions: [
          dialogButton('Cancel', onPressed: close, isOutline: true),
          dialogButton('OK', onPressed: submit),
        ],
        onSubmit: submit,
        onCancel: close,
      );
    });
  }

  Future<void> _switchTo(TailscaleProfile target) async {
    setState(() => _switching = true);
    final ok = await Tailscale.switchProfile(target.id);
    if (!mounted) return;
    setState(() {
      _switching = false;
      if (ok) _selectedProfileId = null;
    });
    if (!ok) showToast(translate('Failed to switch the Tailscale profile'));
    await _refresh();
    // The backend can take a few seconds to come up on the new tailnet.
    for (var i = 0; ok && i < 8; i++) {
      if (!mounted || _snapshot?.state == TailscaleState.running) break;
      await Future.delayed(const Duration(seconds: 1));
      if (!mounted) break;
      await _refresh();
    }
  }

  void _onCardTap(OnPremEntry entry, bool inactiveProfile) {
    if (inactiveProfile) {
      showToast(translate('Switch to this network first'));
      return;
    }
    final device = entry.device;
    final ip = entry.connectIp;
    if (device == null) {
      showToast(translate('Not found in this network'));
      return;
    }
    if (ip == null) {
      showToast(translate('Unreachable'));
      return;
    }
    switch (_rustDeskState(device)) {
      case _RustDeskState.directOff:
        showPublicPeerHelp(entry.name, PublicPeerProblem.directOff);
        return;
      case _RustDeskState.missing:
        showPublicPeerHelp(entry.name, PublicPeerProblem.missing);
        return;
      case _RustDeskState.unreachable:
        showPublicPeerHelp(entry.name, PublicPeerProblem.offline);
        return;
      default:
        break;
    }
    _saveRecords(recordOnPremConnection(_records, _recordProfile, device,
        DateTime.now().millisecondsSinceEpoch));
    connect(context, ip);
  }

  void _toggleFavorite(OnPremEntry entry, TailscaleProfile profile) {
    _saveRecords(
        toggleOnPremFavorite(_records, profile, entry.name, entry.connectIp));
  }

  Widget _buildCard(BuildContext context, OnPremEntry entry,
      {bool inactiveProfile = false}) {
    final device = entry.device;
    final dimmed = device == null;
    final profile = inactiveProfile
        ? _profileForRecord(entry.record) ?? _recordProfile
        : _recordProfile;
    final theme = Theme.of(context);
    final String subtitle;
    if (device == null) {
      subtitle = inactiveProfile
          ? (entry.record?.lastIp ?? '')
          : translate('Not found in this network');
    } else {
      subtitle = _tailscaleLine(device);
    }
    final rustDeskId = device == null ? null : _probe[device.id]?.id;
    final card = Container(
      width: _kCardWidth,
      padding: const EdgeInsets.fromLTRB(12, 10, 4, 10),
      decoration: BoxDecoration(
        color: theme.colorScheme.background,
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: theme.dividerColor),
      ),
      child: Row(
        children: [
          Icon(_osIcon(device?.os ?? ''), size: 32, color: theme.hintColor),
          const SizedBox(width: 10),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(entry.name,
                    overflow: TextOverflow.ellipsis,
                    style: const TextStyle(fontWeight: FontWeight.bold)),
                Text(subtitle,
                    overflow: TextOverflow.ellipsis,
                    style: TextStyle(fontSize: 12, color: theme.hintColor)),
                if (device != null) const SizedBox(height: 4),
                if (device != null) _buildStates(device),
                if (rustDeskId != null && rustDeskId.isNotEmpty)
                  Text('${translate('ID')} $rustDeskId',
                      overflow: TextOverflow.ellipsis,
                      style: TextStyle(fontSize: 12, color: theme.hintColor)),
              ],
            ),
          ),
          IconButton(
            iconSize: 18,
            splashRadius: 16,
            icon: Icon(entry.isFavorite ? Icons.star : Icons.star_border,
                color: entry.isFavorite ? Colors.amber : theme.hintColor),
            tooltip: translate(entry.isFavorite
                ? 'Remove from Favorites'
                : 'Add to Favorites'),
            onPressed: () => _toggleFavorite(entry, profile),
          ),
        ],
      ),
    );
    return Opacity(
      opacity: dimmed ? 0.45 : 1,
      child: InkWell(
        borderRadius: BorderRadius.circular(8),
        onTap: () => _onCardTap(entry, inactiveProfile),
        child: device == null
            ? card
            : Tooltip(
                message: _deviceTooltip(device),
                waitDuration: const Duration(milliseconds: 500),
                child: card,
              ),
      ),
    );
  }

  _RustDeskState _rustDeskState(TailscaleDevice device) {
    final reach = _reach[device.id] ?? RustDeskReachability.checking;
    if (reach == RustDeskReachability.reachable) return _RustDeskState.reachable;
    if (reach == RustDeskReachability.checking) return _RustDeskState.checking;
    // Only online peers with an IPv4 are probed.
    if (!device.online || device.ipv4 == null) {
      return _RustDeskState.unreachable;
    }
    if (!_probe.containsKey(device.id)) return _RustDeskState.checking;
    return _probe[device.id] == null
        ? _RustDeskState.missing
        : _RustDeskState.directOff;
  }

  String _tailscaleLine(TailscaleDevice device) {
    final lastSeen = device.lastSeen;
    if (!device.online && lastSeen != null) {
      return translate('Last seen {${_formatTime(lastSeen)}}');
    }
    final parts = <String>[
      if (device.ipv4 != null) device.ipv4!,
      if (device.curAddr.isNotEmpty)
        translate('Direct')
      else if (device.relay.isNotEmpty)
        translate('Relayed via {${device.relay}}'),
    ];
    return parts.join(' · ');
  }

  String _deviceTooltip(TailscaleDevice device) {
    final peer = _probe[device.id];
    return [
      if (device.dnsName.isNotEmpty) device.dnsName,
      if (device.ips.isNotEmpty) 'IP: ${device.ips.join(', ')}',
      if (device.curAddr.isNotEmpty) '${translate('Direct')}: ${device.curAddr}',
      if (device.tags.isNotEmpty)
        '${translate('Tags')}: ${device.tags.join(', ')}'
      else if (device.owner.isNotEmpty)
        '${translate('Owner')}: ${device.owner}',
      if (device.os.isNotEmpty) 'OS: ${device.os}',
      if (peer != null && peer.id.isNotEmpty) 'RustDesk ${translate('ID')}: ${peer.id}',
      if (peer != null && peer.hostname.isNotEmpty)
        '${translate('Hostname')}: ${peer.hostname}',
      if (_rustDeskState(device) == _RustDeskState.missing)
        translate('Or RustDesk or its LAN discovery is turned off'),
    ].join('\n');
  }

  String _formatTime(DateTime time) {
    final t = time.toLocal();
    String two(int n) => n.toString().padLeft(2, '0');
    return '${t.year}-${two(t.month)}-${two(t.day)} ${two(t.hour)}:${two(t.minute)}';
  }

  TailscaleProfile? _profileForRecord(OnPremRecord? record) {
    if (record == null) return null;
    for (final p in _snapshot?.profiles ?? const <TailscaleProfile>[]) {
      if (p.tailnet == record.tailnet) return p;
    }
    return null;
  }

  Widget _buildStates(TailscaleDevice device) {
    final (String label, Color color) = switch (_rustDeskState(device)) {
      _RustDeskState.reachable => ('Reachable', Colors.green),
      _RustDeskState.directOff => (
          'RustDesk installed, direct access off',
          Colors.orange
        ),
      _RustDeskState.missing => ('No RustDesk', Colors.redAccent),
      _RustDeskState.unreachable => ('Unreachable', Colors.redAccent),
      _RustDeskState.checking => ('Checking', Colors.grey),
    };
    // The RustDesk state gets its own line: the longer labels do not fit
    // beside the online state in a card this narrow.
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            Container(
              width: 7,
              height: 7,
              decoration: BoxDecoration(
                shape: BoxShape.circle,
                color: device.online ? Colors.green : Colors.grey,
              ),
            ),
            const SizedBox(width: 4),
            Text(translate(device.online ? 'Online' : 'Offline'),
                style: const TextStyle(fontSize: 12)),
          ],
        ),
        Text(translate(label),
            maxLines: 2,
            overflow: TextOverflow.ellipsis,
            style: TextStyle(fontSize: 12, color: color)),
      ],
    );
  }

  IconData _osIcon(String os) {
    switch (os.toLowerCase()) {
      case 'macos':
        return Icons.desktop_mac;
      case 'windows':
        return Icons.desktop_windows;
      case 'linux':
        return Icons.dns;
      case 'android':
        return Icons.phone_android;
      case 'ios':
        return Icons.phone_iphone;
      default:
        return Icons.devices_other;
    }
  }
}
