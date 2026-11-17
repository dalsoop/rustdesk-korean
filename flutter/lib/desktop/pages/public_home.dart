// Korean fork, RustDesk Public flavor: the right pane of the main window.
// Tabs 온프레미스 (this Mac's Tailscale tailnets) | 인터넷 (stock ConnectionPage).

import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hbb/common.dart';
import 'package:flutter_hbb/common/formatter/id_formatter.dart';
import 'package:flutter_hbb/common/public_onprem.dart';
import 'package:flutter_hbb/common/tailscale.dart';
import 'package:flutter_hbb/consts.dart';
import 'package:flutter_hbb/desktop/pages/connection_page.dart';
import 'package:flutter_hbb/desktop/pages/public_onboarding.dart';
import 'package:flutter_hbb/models/platform_model.dart';
import 'package:get/get.dart';

const double _kCardWidth = 240;
const Set<OnPremSection> _kFoldedByDefault = {
  OnPremSection.missing,
  OnPremSection.offline,
};
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
  // Without Tailscale on this Mac the on-prem tab is hidden and the home is the
  // internet tab alone; it appears by itself once Tailscale is installed.
  bool _tsInstalled = Tailscale.findBinary() != null;
  int get _shownTab => _tsInstalled ? _tab : _internetTab;
  TailscaleSnapshot? _snapshot;
  bool _loading = false;
  bool _switching = false;
  // Sections the user opened or closed against their default.
  final Set<OnPremSection> _flipped = {};
  final TextEditingController _search = TextEditingController();
  late final OnPremRustDeskMemos _memos;
  int _pendingChecks = 0;
  bool _memosDirty = false;
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
    _memos = OnPremRustDeskMemos.decode(
        bind.getLocalFlutterOption(k: kOptionPublicOnPremRustDeskMemos));
    _search.addListener(() => setState(() {}));
    _refresh();
    _setupPoll = Timer.periodic(_kSetupPollInterval, (_) {
      if (!_tsInstalled) {
        if (Tailscale.findBinary() != null) {
          setState(() => _tsInstalled = true);
          _refresh();
        }
        return;
      }
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
      WidgetsBinding.instance.addPostFrameCallback((_) => showPublicWelcome(
          _onWelcomeChoice,
          teamDevices: _tsInstalled));
    }
  }

  @override
  void dispose() {
    _setupPoll?.cancel();
    _search.dispose();
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
      _tsInstalled = Tailscale.findBinary() != null;
      if (!ids.contains(_selectedProfileId)) {
        _selectedProfileId = snapshot.activeProfile?.id ??
            (snapshot.profiles.isEmpty ? null : snapshot.profiles.first.id);
      }
      _reach.clear();
      _probe.clear();
      _pendingChecks = 0;
    });
    if (snapshot.state != TailscaleState.running) return;
    // The devices belong to the running tailnet, whichever profile is picked.
    final tailnet = snapshot.activeProfile?.tailnet ?? '';
    for (final d in snapshot.devices) {
      final ip = d.ipv4;
      if (ip == null) {
        _reach[d.id] = RustDeskReachability.unreachable;
        continue;
      }
      _reach[d.id] = RustDeskReachability.checking;
      _pendingChecks++;
      Tailscale.checkRustDesk(ip).then((result) {
        if (!mounted || seq != _readSeq) return;
        setState(() => _reach[d.id] = result);
        _settle(d, tailnet);
      });
      // An offline peer cannot answer; skipping it keeps the bridge's
      // worker threads free for the peers that can.
      if (!d.online) continue;
      _pendingChecks++;
      bind.mainPublicProbeRustdesk(ip: ip).then((out) {
        if (!mounted || seq != _readSeq) return;
        setState(() => _probe[d.id] = parseRustDeskProbe(out));
        _settle(d, tailnet);
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
            index: _shownTab,
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
          offstage: _shownTab != _onPremTab,
          child: _buildOnPremStatus(),
        ),
        // Kept mounted on both tabs: it also polls the service state for
        // the rest of the app.
        Offstage(
          offstage: _shownTab != _internetTab,
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
        if (_tsInstalled) tab(_onPremTab, 'On-premises'),
        if (_tsInstalled) tab(_internetTab, 'Internet'),
        if (!_tsInstalled)
          // Flexible must stay the Row's direct child: the margin goes inside it.
          Flexible(
            child: Text(translate('public_no_tailscale_hint'),
                    overflow: TextOverflow.ellipsis,
                    style: TextStyle(
                        fontSize: 12, color: Theme.of(context).hintColor))
                .marginOnly(left: 6, top: 12, bottom: 12),
          ),
        const Spacer(),
        const PublicAppLabel(),
        IconButton(
          tooltip: translate('Show the welcome guide'),
          icon: const Icon(Icons.help_outline, size: 20),
          onPressed: () =>
              showPublicWelcome(_onWelcomeChoice, teamDevices: _tsInstalled),
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
            if (snapshot.state == TailscaleState.running)
              _buildSearchField().marginOnly(left: 12),
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
    final query = _search.text.trim();
    final groups = <OnPremSection, List<OnPremEntry>>{};
    for (final e in entries) {
      final device = e.device;
      final id = device == null ? '' : _rustDeskIdOf(device);
      if (!onPremEntryMatches(e, query, rustDeskId: id)) continue;
      final section = onPremSectionOf(
          e, device == null ? null : _shownRustDesk(device));
      groups.putIfAbsent(section, () => []).add(e);
    }
    final hint = TextStyle(fontSize: 13, color: Theme.of(context).hintColor);
    final noneUsable = const [
      OnPremSection.favorites,
      OnPremSection.ready,
      OnPremSection.directOff,
      OnPremSection.checking,
    ].every((s) => !groups.containsKey(s));
    return SingleChildScrollView(
      padding: const EdgeInsets.fromLTRB(12, 0, 12, 12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          if (groups.isEmpty)
            Text(translate('No devices match'), style: hint).marginOnly(top: 12),
          if (groups.isNotEmpty && query.isEmpty && noneUsable)
            Text(translate('public_no_ready_tip'), style: hint)
                .marginOnly(top: 12),
          for (final s in OnPremSection.values)
            if (groups[s] != null)
              ..._buildSection(context, s, groups[s]!, query.isNotEmpty),
        ],
      ),
    );
  }

  Widget _buildSearchField() {
    return SizedBox(
      width: 220,
      child: TextField(
        controller: _search,
        style: const TextStyle(fontSize: 13),
        decoration: InputDecoration(
          isDense: true,
          prefixIcon: const Icon(Icons.search, size: 18),
          prefixIconConstraints:
              const BoxConstraints(minWidth: 32, minHeight: 32),
          hintText: translate('Search by name, IP or ID'),
          contentPadding:
              const EdgeInsets.symmetric(horizontal: 8, vertical: 8),
          border: const OutlineInputBorder(),
          suffixIcon: _search.text.isEmpty
              ? null
              : IconButton(
                  icon: const Icon(Icons.clear, size: 16),
                  splashRadius: 14,
                  onPressed: _search.clear,
                ),
        ),
      ),
    );
  }

  List<Widget> _buildSection(BuildContext context, OnPremSection section,
      List<OnPremEntry> entries, bool searching) {
    final theme = Theme.of(context);
    final open = searching ||
        _kFoldedByDefault.contains(section) == _flipped.contains(section);
    final (IconData icon, Color color, String title) = switch (section) {
      OnPremSection.favorites => (Icons.star, Colors.amber, 'Favorites'),
      OnPremSection.ready => (
          Icons.check_circle,
          Colors.green,
          'RustDesk installed, ready to connect'
        ),
      OnPremSection.directOff => (
          Icons.do_not_disturb_on,
          Colors.orange,
          'RustDesk installed, direct access off'
        ),
      OnPremSection.checking => (
          Icons.hourglass_empty,
          theme.hintColor,
          'Checking'
        ),
      OnPremSection.missing => (
          Icons.remove_circle_outline,
          Colors.redAccent,
          'No RustDesk'
        ),
      OnPremSection.offline => (Icons.cloud_off, theme.hintColor, 'Offline'),
    };
    final header = InkWell(
      borderRadius: BorderRadius.circular(4),
      onTap: searching
          ? null
          : () => setState(() {
                if (!_flipped.remove(section)) _flipped.add(section);
              }),
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 4),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(open ? Icons.expand_more : Icons.chevron_right,
                size: 18, color: theme.hintColor),
            const SizedBox(width: 2),
            Icon(icon, size: 16, color: color),
            const SizedBox(width: 6),
            Text('${translate(title)} (${entries.length})',
                style: const TextStyle(
                    fontSize: 14, fontWeight: FontWeight.bold)),
          ],
        ),
      ),
    ).marginOnly(top: 14, bottom: 6);
    if (!open) return [header];

    Widget cards(List<OnPremEntry> list) => Wrap(
          spacing: 12,
          runSpacing: 12,
          children: list.map((e) => _buildCard(context, e)).toList(),
        );
    final byKind = <OnPremDeviceKind, List<OnPremEntry>>{};
    for (final e in entries) {
      byKind
          .putIfAbsent(onPremDeviceKindOf(e.device?.os ?? ''), () => [])
          .add(e);
    }
    if (byKind.length < 2) return [header, cards(entries)];
    return [
      header,
      for (final k in OnPremDeviceKind.values)
        if (byKind[k] != null) ...[
          Text('${translate(_kindTitle(k))} (${byKind[k]!.length})',
                  style: TextStyle(fontSize: 12, color: theme.hintColor))
              .marginOnly(left: 4, top: 4, bottom: 6),
          cards(byKind[k]!).marginOnly(bottom: 8),
        ],
    ];
  }

  String _kindTitle(OnPremDeviceKind kind) => switch (kind) {
        OnPremDeviceKind.computer => 'Computers',
        OnPremDeviceKind.server => 'Linux and servers',
        OnPremDeviceKind.mobile => 'Phones and tablets',
        OnPremDeviceKind.other => 'Other kinds',
      };

  OnPremRustDesk? _liveRustDesk(TailscaleDevice device) =>
      switch (_rustDeskState(device)) {
        _RustDeskState.reachable => OnPremRustDesk.reachable,
        _RustDeskState.directOff => OnPremRustDesk.directOff,
        _RustDeskState.missing => OnPremRustDesk.missing,
        _ => null,
      };

  /// The live state, or the remembered one while an online device is still
  /// being checked.
  OnPremRustDesk? _shownRustDesk(TailscaleDevice device) {
    final live = _liveRustDesk(device);
    if (live != null) return live;
    if (!device.online ||
        _rustDeskState(device) != _RustDeskState.checking) {
      return null;
    }
    final remembered = _memos.get(_recordProfile.tailnet, device.name)?.state;
    // The port check already failed; only the probe can still tell which.
    if (remembered == OnPremRustDesk.reachable &&
        _reach[device.id] == RustDeskReachability.unreachable) {
      return null;
    }
    return remembered;
  }

  String _rustDeskIdOf(TailscaleDevice device) {
    final probed = _probe[device.id]?.id ?? '';
    if (probed.isNotEmpty) return probed;
    if (_shownRustDesk(device) == OnPremRustDesk.missing) return '';
    return _memos.get(_recordProfile.tailnet, device.name)?.id ?? '';
  }

  /// Remembers a finished check and saves the memos once every check of this
  /// read is done.
  void _settle(TailscaleDevice device, String tailnet) {
    final live = _liveRustDesk(device);
    if (live != null) {
      final probed = _probe[device.id]?.id ?? '';
      final id = live == OnPremRustDesk.missing
          ? ''
          : (probed.isNotEmpty
              ? probed
              : _memos.get(tailnet, device.name)?.id ?? '');
      if (_memos.put(
          tailnet, device.name, OnPremRustDeskMemo(state: live, id: id))) {
        _memosDirty = true;
      }
    }
    _pendingChecks--;
    if (_pendingChecks <= 0 && _memosDirty) {
      _memosDirty = false;
      bind.setLocalFlutterOption(
          k: kOptionPublicOnPremRustDeskMemos, v: _memos.encode());
    }
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
      subtitle = inactiveProfile ? '' : translate('Not found in this network');
    } else {
      // An online device's path goes on its status line.
      subtitle = device.online ? '' : _tailscaleLine(device);
    }
    final ip = device?.ipv4 ?? entry.record?.lastIp ?? '';
    final rustDeskId = device == null ? '' : _rustDeskIdOf(device);
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
                if (ip.isNotEmpty) _copyLine(theme, 'IP', ip, ip),
                if (subtitle.isNotEmpty)
                  Text(subtitle,
                      overflow: TextOverflow.ellipsis,
                      style: TextStyle(fontSize: 12, color: theme.hintColor)),
                if (device != null) const SizedBox(height: 4),
                if (device != null) _buildStates(device),
                if (rustDeskId.isNotEmpty)
                  _copyLine(theme, translate('ID'), formatID(rustDeskId),
                      rustDeskId),
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

  Widget _copyLine(ThemeData theme, String label, String shown, String value) {
    return Row(
      children: [
        Flexible(
          child: Text('$label $shown',
              overflow: TextOverflow.ellipsis,
              style: TextStyle(fontSize: 12, color: theme.hintColor)),
        ),
        Tooltip(
          message: translate('Copy'),
          child: InkWell(
            borderRadius: BorderRadius.circular(4),
            onTap: () {
              Clipboard.setData(ClipboardData(text: value));
              showToast(translate('Copied'));
            },
            child: Padding(
              padding: const EdgeInsets.all(3),
              child: Icon(Icons.copy, size: 13, color: theme.hintColor),
            ),
          ),
        ),
      ],
    );
  }

  Widget _buildStates(TailscaleDevice device) {
    var state = _rustDeskState(device);
    if (state == _RustDeskState.checking) {
      state = switch (_shownRustDesk(device)) {
        OnPremRustDesk.reachable => _RustDeskState.reachable,
        OnPremRustDesk.directOff => _RustDeskState.directOff,
        OnPremRustDesk.missing => _RustDeskState.missing,
        null => _RustDeskState.checking,
      };
    }
    final (String label, Color color) = switch (state) {
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
            Flexible(
              child: Text(
                  device.online && _tailscaleLine(device).isNotEmpty
                      ? '${translate('Online')} · ${_tailscaleLine(device)}'
                      : translate(device.online ? 'Online' : 'Offline'),
                  overflow: TextOverflow.ellipsis,
                  style: const TextStyle(fontSize: 12)),
            ),
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
