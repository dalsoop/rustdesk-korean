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

  test('status json tells sign-in and approval apart from a stopped Tailscale',
      () {
    TailscaleState stateOf(String backend) =>
        parseTailscaleStatusJson('{"BackendState": "$backend"}').state;
    expect(stateOf('NeedsLogin'), TailscaleState.needsLogin);
    expect(stateOf('NeedsMachineAuth'), TailscaleState.needsApproval);
    expect(stateOf('Stopped'), TailscaleState.stopped);
  });

  test('status json gives each peer its Tailscale details', () {
    const output = '''
{
  "BackendState": "Running",
  "Self": {"ID": "n1", "UserID": 7},
  "User": {
    "7": {"ID": 7, "LoginName": "urit245@gmail.com"},
    "8": {"ID": 8, "LoginName": "medicalo2026@gmail.com"}
  },
  "Peer": {
    "nodekey:a": {
      "ID": "n2",
      "DNSName": "m5m2.tail1234.ts.net.",
      "OS": "macOS",
      "Online": true,
      "TailscaleIPs": ["100.91.226.87", "fd7a:115c:a1e0::5"],
      "CurAddr": "192.168.0.12:41641",
      "Relay": "tok",
      "UserID": 8,
      "LastSeen": "0001-01-01T00:00:00Z"
    },
    "nodekey:b": {
      "ID": "n3",
      "DNSName": "nas.tail1234.ts.net.",
      "OS": "linux",
      "Online": false,
      "TailscaleIPs": ["100.64.0.3"],
      "CurAddr": "",
      "Relay": "tok",
      "UserID": 7,
      "Tags": ["tag:server"],
      "LastSeen": "2026-11-07T10:00:00Z"
    }
  }
}
''';
    final devices = parseTailscaleStatusJson(output).devices;
    final mac = devices[0];
    expect(mac.dnsName, 'm5m2.tail1234.ts.net');
    expect(mac.ips, ['100.91.226.87', 'fd7a:115c:a1e0::5']);
    expect(mac.curAddr, '192.168.0.12:41641');
    expect(mac.owner, 'medicalo2026@gmail.com');
    expect(mac.lastSeen, isNull);
    final nas = devices[1];
    expect(nas.curAddr, '');
    expect(nas.relay, 'tok');
    expect(nas.owner, 'urit245@gmail.com');
    expect(nas.tags, ['tag:server']);
    expect(nas.lastSeen, DateTime.utc(2026, 11, 7, 10));
  });
}
