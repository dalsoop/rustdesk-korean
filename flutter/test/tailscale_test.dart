import 'package:flutter_hbb/common/tailscale.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('switch --list output gives profiles with the active one marked', () {
    const output = '''
ID    Tailnet                 Account
e4e8  urit245@gmail.com       urit245@gmail.com*
8a1d  medicalo2026@gmail.com  medicalo2026@gmail.com
''';
    final profiles = parseTailscaleProfiles(output);
    expect(profiles.map((p) => p.id), ['e4e8', '8a1d']);
    expect(profiles[0].tailnet, 'urit245@gmail.com');
    expect(profiles[0].account, 'urit245@gmail.com');
    expect(profiles[0].active, isTrue);
    expect(profiles[1].account, 'medicalo2026@gmail.com');
    expect(profiles[1].active, isFalse);
  });

  test('status json lists peers without this Mac, by short name and IPv4', () {
    const output = '''
{
  "BackendState": "Running",
  "Self": {
    "ID": "n1",
    "HostName": "m5m1",
    "DNSName": "m5m1.tail1234.ts.net.",
    "OS": "macOS",
    "Online": true,
    "TailscaleIPs": ["100.64.0.1", "fd7a:115c:a1e0::1"]
  },
  "Peer": {
    "nodekey:a": {
      "ID": "n2",
      "HostName": "Galaxy S24",
      "DNSName": "galaxy-s24.tail1234.ts.net.",
      "OS": "android",
      "Online": false,
      "TailscaleIPs": ["fd7a:115c:a1e0::2", "100.64.0.2"]
    },
    "nodekey:b": {
      "ID": "n3",
      "HostName": "nas",
      "DNSName": "",
      "OS": "linux",
      "Online": true,
      "TailscaleIPs": ["100.64.0.3"]
    },
    "nodekey:self": {
      "ID": "n1",
      "HostName": "m5m1",
      "DNSName": "m5m1.tail1234.ts.net.",
      "OS": "macOS",
      "Online": true,
      "TailscaleIPs": ["100.64.0.1"]
    }
  }
}
''';
    final status = parseTailscaleStatusJson(output);
    expect(status.state, TailscaleState.running);
    expect(status.devices.map((d) => d.name), ['galaxy-s24', 'nas']);
    final phone = status.devices[0];
    expect(phone.os, 'android');
    expect(phone.online, isFalse);
    expect(phone.ipv4, '100.64.0.2');
    expect(status.devices[1].ipv4, '100.64.0.3');
  });
}
