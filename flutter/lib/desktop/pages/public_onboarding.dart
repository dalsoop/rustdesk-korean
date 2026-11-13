// Korean fork, RustDesk Public flavor: what a new user needs on the home
// screen. A first-run guide that picks the start tab, the left-pane note that
// this app only makes outgoing connections, a next step for every Tailscale
// state, the reason a device card cannot be used, and the internet sign-in.

import 'package:flutter/material.dart';
import 'package:flutter_hbb/common.dart';
import 'package:flutter_hbb/common/tailscale.dart';
import 'package:flutter_hbb/common/widgets/login.dart';
import 'package:flutter_hbb/models/platform_model.dart';
import 'package:get/get.dart';
import 'package:package_info_plus/package_info_plus.dart';
import 'package:url_launcher/url_launcher.dart';

const String kOptionPublicWelcomeDone = 'public-welcome-done';
const String kOptionPublicStartTab = 'public-start-tab';
const String _kStartTabInternet = 'internet';
const String _kStartTabOnPrem = 'onprem';
const String _kTailscaleDownloadUrl = 'https://tailscale.com/download/mac';

bool get publicWelcomeDone =>
    bind.getLocalFlutterOption(k: kOptionPublicWelcomeDone) == 'Y';

bool get publicStartsOnInternet =>
    bind.getLocalFlutterOption(k: kOptionPublicStartTab) == _kStartTabInternet;

/// Asks how the user will connect; [onChoose] gets true for the internet tab.
/// Closing it without a choice keeps the start tab as it was.
void showPublicWelcome(void Function(bool internet) onChoose) {
  gFFI.dialogManager.show((setState, close, context) {
    void finish(bool? internet) {
      bind.setLocalFlutterOption(k: kOptionPublicWelcomeDone, v: 'Y');
      close();
      if (internet == null) return;
      bind.setLocalFlutterOption(
          k: kOptionPublicStartTab,
          v: internet ? _kStartTabInternet : _kStartTabOnPrem);
      onChoose(internet);
    }

    final hint = TextStyle(fontSize: 12, color: Theme.of(context).hintColor);
    return CustomAlertDialog(
      title: Text(translate('Welcome to RustDesk Public')),
      contentBoxConstraints: const BoxConstraints(maxWidth: 460),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(translate('public_welcome_tip')),
          const SizedBox(height: 14),
          _choiceCard(context, Icons.lan_outlined, 'Connect to team devices',
              'public_onprem_choice_tip', () => finish(false)),
          const SizedBox(height: 10),
          _choiceCard(context, Icons.public, 'Connect over the internet',
              'public_internet_choice_tip', () => finish(true)),
          const SizedBox(height: 14),
          Text(translate('public_incoming_tip'), style: hint),
        ],
      ),
      actions: [
        dialogButton('Later', onPressed: () => finish(null), isOutline: true),
      ],
      onCancel: () => finish(null),
    );
  });
}

Widget _choiceCard(BuildContext context, IconData icon, String title,
    String tip, VoidCallback onTap) {
  final theme = Theme.of(context);
  return InkWell(
    borderRadius: BorderRadius.circular(8),
    onTap: onTap,
    child: Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: theme.dividerColor),
      ),
      child: Row(
        children: [
          Icon(icon, size: 28, color: MyTheme.accent),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(translate(title),
                    style: const TextStyle(fontWeight: FontWeight.bold)),
                const SizedBox(height: 4),
                Text(translate(tip),
                    style: TextStyle(fontSize: 12, color: theme.hintColor)),
              ],
            ),
          ),
          Icon(Icons.chevron_right, color: theme.hintColor),
        ],
      ),
    ),
  );
}

/// "RustDesk Public 1.5.0 (ko.N)" in the tab bar, so this window is never taken for the main
/// RustDesk app. The fork build number is upstream build * 100 + N (pubspec.yaml).
class PublicAppLabel extends StatelessWidget {
  const PublicAppLabel({Key? key}) : super(key: key);

  static Future<String> _version() async {
    final version = await bind.mainGetVersion();
    final build = int.tryParse((await PackageInfo.fromPlatform()).buildNumber);
    return build != null && build >= 100 ? '$version (ko.${build % 100})' : version;
  }

  @override
  Widget build(BuildContext context) {
    final hint = Theme.of(context).hintColor;
    return FutureBuilder<String>(
      future: _version(),
      builder: (context, snapshot) => Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          const Icon(Icons.public, size: 14, color: Color(0xFFFF8500)),
          const SizedBox(width: 4),
          const Text('RustDesk Public',
              style: TextStyle(fontSize: 12, fontWeight: FontWeight.bold)),
          if (snapshot.hasData) const SizedBox(width: 6),
          if (snapshot.hasData)
            Text(snapshot.data!, style: TextStyle(fontSize: 12, color: hint)),
        ],
      ),
    );
  }
}

/// Left pane text in place of the incoming ID and password board.
class PublicIntroCard extends StatelessWidget {
  const PublicIntroCard({Key? key}) : super(key: key);

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(left: 20, right: 16, top: 16, bottom: 5),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(translate('Outgoing connections only'),
              style: Theme.of(context).textTheme.titleLarge),
          const SizedBox(height: 10),
          Text(translate('public_incoming_tip'),
              style: Theme.of(context).textTheme.bodySmall),
        ],
      ),
    );
  }
}

/// The next step for an on-prem tab that has no device to show. [state] is
/// [TailscaleState.running] when the tailnet simply has no other device.
class PublicTailscaleGuide extends StatelessWidget {
  final TailscaleState state;
  // Null while a read is already running.
  final VoidCallback? onCheckAgain;
  final VoidCallback onUseInternet;

  const PublicTailscaleGuide({
    Key? key,
    required this.state,
    required this.onCheckAgain,
    required this.onUseInternet,
  }) : super(key: key);

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    // notFound also covers a CLI that did not answer in time.
    final installed = Tailscale.findBinary() != null;
    final (IconData icon, String title, String tip) = switch (state) {
      TailscaleState.notFound when installed => (
          Icons.sync_problem,
          'Tailscale is not responding',
          'public_ts_not_responding_tip'
        ),
      TailscaleState.notFound => (
          Icons.download_outlined,
          'Tailscale is not installed',
          'public_ts_install_tip'
        ),
      TailscaleState.stopped => (
          Icons.power_settings_new,
          'Tailscale is turned off',
          'public_ts_off_tip'
        ),
      TailscaleState.needsLogin => (
          Icons.login,
          'Sign in to Tailscale',
          'public_ts_login_tip'
        ),
      TailscaleState.needsApproval => (
          Icons.hourglass_empty,
          'Waiting for administrator approval',
          'public_ts_approval_tip'
        ),
      TailscaleState.running => (
          Icons.devices_other,
          'No other devices in this network',
          'public_ts_empty_tip'
        ),
    };
    final canOpenApp = state == TailscaleState.stopped ||
        state == TailscaleState.needsLogin ||
        (state == TailscaleState.notFound && installed);
    return Center(
      child: SingleChildScrollView(
        padding: const EdgeInsets.all(24),
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 420),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(icon, size: 40, color: theme.hintColor),
              const SizedBox(height: 12),
              Text(translate(title),
                  textAlign: TextAlign.center,
                  style: const TextStyle(
                      fontSize: 16, fontWeight: FontWeight.bold)),
              const SizedBox(height: 8),
              Text(translate(tip),
                  textAlign: TextAlign.center,
                  style: TextStyle(fontSize: 13, color: theme.hintColor)),
              const SizedBox(height: 16),
              Wrap(
                spacing: 8,
                runSpacing: 8,
                alignment: WrapAlignment.center,
                children: [
                  if (state == TailscaleState.notFound && !installed)
                    ElevatedButton(
                      onPressed: () =>
                          launchUrl(Uri.parse(_kTailscaleDownloadUrl)),
                      child: Text(translate('Download Tailscale')),
                    ),
                  if (canOpenApp)
                    ElevatedButton(
                      onPressed: () async {
                        if (!await Tailscale.openApp()) {
                          showToast(translate('Tailscale not found'));
                        }
                      },
                      child: Text(translate('Open Tailscale')),
                    ),
                  OutlinedButton(
                    onPressed: onCheckAgain,
                    child: Text(translate('Check again')),
                  ),
                  if (state != TailscaleState.running)
                    TextButton(
                      onPressed: onUseInternet,
                      child: Text(translate('Connect over the internet instead')),
                    ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }
}

enum PublicPeerProblem { directOff, missing, offline }

/// Says why [deviceName] cannot take a connection and what to do about it.
void showPublicPeerHelp(String deviceName, PublicPeerProblem problem) {
  final tip = switch (problem) {
    PublicPeerProblem.directOff => 'public_direct_off_help',
    PublicPeerProblem.missing => 'public_missing_help',
    PublicPeerProblem.offline => 'public_offline_help',
  };
  gFFI.dialogManager.show((setState, close, context) {
    return CustomAlertDialog(
      title: Text(deviceName),
      content: Text(translate(tip)),
      actions: [dialogButton('OK', onPressed: close)],
      onSubmit: close,
      onCancel: close,
    );
  });
}

/// Shown above the internet tab while no account is signed in: the public
/// server needs the controlling side to sign in.
class PublicInternetLoginBanner extends StatelessWidget {
  const PublicInternetLoginBanner({Key? key}) : super(key: key);

  @override
  Widget build(BuildContext context) {
    return Obx(() {
      if (gFFI.userModel.userName.value.isNotEmpty) {
        return const SizedBox.shrink();
      }
      return Container(
        margin: const EdgeInsets.fromLTRB(12, 12, 12, 0),
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
        decoration: BoxDecoration(
          borderRadius: BorderRadius.circular(8),
          border: Border.all(color: MyTheme.accent),
        ),
        child: Row(
          children: [
            const Icon(Icons.info_outline, color: MyTheme.accent),
            const SizedBox(width: 10),
            Expanded(
              child: Text(translate('public_internet_login_tip'),
                  style: const TextStyle(fontSize: 13)),
            ),
            const SizedBox(width: 8),
            ElevatedButton(
              onPressed: loginDialog,
              child: Text(translate('Login')),
            ),
          ],
        ),
      );
    });
  }
}
