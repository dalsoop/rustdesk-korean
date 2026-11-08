// Korean fork, RustDesk Public flavor: the right pane of the main window.
// Tabs 온프레미스 (this Mac's Tailscale tailnets) | 인터넷 (stock ConnectionPage).

import 'package:flutter/material.dart';
import 'package:flutter_hbb/common.dart';
import 'package:flutter_hbb/common/public_onprem.dart';
import 'package:flutter_hbb/common/tailscale.dart';
import 'package:flutter_hbb/consts.dart';
import 'package:flutter_hbb/desktop/pages/connection_page.dart';
import 'package:flutter_hbb/models/platform_model.dart';
import 'package:get/get.dart';

const double _kCardWidth = 220;
const double _kStatusBarHeight = 42;

class PublicHomePage extends StatefulWidget {
  const PublicHomePage({Key? key}) : super(key: key);

  @override
  State<PublicHomePage> createState() => _PublicHomePageState();
}

class _PublicHomePageState extends State<PublicHomePage> {
  static const int _onPremTab = 0;
  static const int _internetTab = 1;

  // Always opens on 온프레미스; the tab is deliberately not remembered.
  int _tab = _onPremTab;
  TailscaleSnapshot? _snapshot;
  bool _loading = false;
  bool _switching = false;
  String? _selectedProfileId;
  int _readSeq = 0;
  final Map<String, RustDeskReachability> _reach = {};
  List<OnPremRecord> _records = [];

  @override
  void initState() {
    super.initState();
    _records = decodeOnPremRecords(
        bind.getLocalFlutterOption(k: kOptionPublicOnPremRecords));
    _refresh();
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
              ConnectionPage(),
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
      ],
    ).marginOnly(left: 12, top: 8);
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
      return Center(child: Text(translate('Tailscale not found')));
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
    if (snapshot.state == TailscaleState.stopped) {
      return Center(child: Text(translate('Tailscale is turned off')));
    }
    final selected = _selectedProfile;
    if (selected != null && !selected.active) {
      return _buildSwitchArea(context, snapshot, selected);
    }
    final entries = buildOnPremEntries(
        snapshot.devices, onPremRecordsFor(_records, _recordProfile.tailnet));
    if (entries.isEmpty) {
      return Center(child: Text(translate('Empty')));
    }
    return SingleChildScrollView(
      padding: const EdgeInsets.all(12),
      child: Wrap(
        spacing: 12,
        runSpacing: 12,
        children: entries.map((e) => _buildCard(context, e)).toList(),
      ),
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
      subtitle = device.os;
    }
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
        child: card,
      ),
    );
  }

  TailscaleProfile? _profileForRecord(OnPremRecord? record) {
    if (record == null) return null;
    for (final p in _snapshot?.profiles ?? const <TailscaleProfile>[]) {
      if (p.tailnet == record.tailnet) return p;
    }
    return null;
  }

  Widget _buildStates(TailscaleDevice device) {
    final reach = _reach[device.id] ?? RustDeskReachability.checking;
    final (String label, Color color) = switch (reach) {
      RustDeskReachability.reachable => ('Reachable', Colors.green),
      RustDeskReachability.unreachable => ('Unreachable', Colors.redAccent),
      RustDeskReachability.checking => ('Checking', Colors.grey),
    };
    return Row(
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
        const SizedBox(width: 8),
        Flexible(
          child: Text(translate(label),
              overflow: TextOverflow.ellipsis,
              style: TextStyle(fontSize: 12, color: color)),
        ),
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
