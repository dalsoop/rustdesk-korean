import 'package:flutter_hbb/common/connect_onprem.dart';
import 'package:flutter_hbb/common/tailscale.dart';
import 'package:flutter_hbb/common/tailscale_range.dart';
import 'package:flutter_test/flutter_test.dart';

const _home = TailscaleProfile(
    id: 'e4e8', tailnet: 'home.example', account: 'a@x.com', active: true);
const _work = TailscaleProfile(
    id: '8a1d', tailnet: 'work.example', account: 'b@x.com', active: false);

TailscaleDevice _device(String name, String ip) =>
    TailscaleDevice(id: name, name: name, os: 'macOS', online: true, ipv4: ip);

void main() {
  test('only IPv4 IDs inside 100.64.0.0/10 count as on-prem', () {
    expect(isTailscaleRangeId('100.64.0.1'), isTrue);
    expect(isTailscaleRangeId('100.127.255.254'), isTrue);
    expect(isTailscaleRangeId('100.101.2.3:21118'), isTrue);
    expect(isTailscaleRangeId('100.128.0.1'), isFalse);
    expect(isTailscaleRangeId('100.63.255.255'), isFalse);
    expect(isTailscaleRangeId('192.168.0.10'), isFalse);
    expect(isTailscaleRangeId('1703636484'), isFalse);
  });

  test('records come back per tailnet and connect to the current IP', () {
    var records = recordOnPremConnection(
        [], _home, _device('studio', '100.64.0.5'), 1000);
    records = toggleOnPremFavorite(records, _home, 'nas', '100.64.0.9');
    records = recordOnPremConnection(
        records, _work, _device('build', '100.70.0.2'), 2000);
    records = decodeOnPremRecords(encodeOnPremRecords(records));

    final entries = buildOnPremEntries(
        [_device('printer', '100.64.0.30'), _device('studio', '100.64.0.77')],
        onPremRecordsFor(records, _home.tailnet));

    expect(entries.map((e) => e.name), ['nas', 'studio', 'printer']);
    final nas = entries[0];
    expect(nas.isFavorite, isTrue);
    expect(nas.missing, isTrue);
    expect(nas.connectIp, isNull);
    final studio = entries[1];
    expect(studio.missing, isFalse);
    expect(studio.record?.lastIp, '100.64.0.5');
    expect(studio.connectIp, '100.64.0.77');
    expect(entries[2].record, isNull);

    expect(onPremRecordsFor(records, _work.tailnet).map((r) => r.deviceName),
        ['build']);
    expect(onPremRecordsFor(records, 'gone.example'), isEmpty);
  });

  test('devices sort into RustDesk sections and remembered results survive',
      () {
    final records = toggleOnPremFavorite([], _home, 'fav', '100.64.0.4');
    const offline = TailscaleDevice(
        id: 'o', name: 'old', os: 'windows', online: false, ipv4: '100.64.0.8');
    final entries = buildOnPremEntries([
      _device('fav', '100.64.0.4'),
      _device('mac', '100.64.0.5'),
      offline,
    ], records);
    final byName = {for (final e in entries) e.name: e};

    expect(onPremSectionOf(byName['fav']!, OnPremRustDesk.missing),
        OnPremSection.favorites);
    expect(onPremSectionOf(byName['mac']!, OnPremRustDesk.reachable),
        OnPremSection.ready);
    expect(onPremSectionOf(byName['mac']!, OnPremRustDesk.directOff),
        OnPremSection.directOff);
    expect(onPremSectionOf(byName['mac']!, OnPremRustDesk.missing),
        OnPremSection.missing);
    expect(onPremSectionOf(byName['mac']!, null), OnPremSection.checking);
    expect(onPremSectionOf(byName['old']!, OnPremRustDesk.directOff),
        OnPremSection.offline);

    expect(onPremDeviceKindOf('macOS'), OnPremDeviceKind.computer);
    expect(onPremDeviceKindOf('linux'), OnPremDeviceKind.server);
    expect(onPremDeviceKindOf('android'), OnPremDeviceKind.mobile);

    expect(onPremEntryMatches(byName['mac']!, '0.5'), isTrue);
    expect(onPremEntryMatches(byName['mac']!, '158 970', rustDeskId: '1589700753'),
        isTrue);
    expect(onPremEntryMatches(byName['mac']!, 'old'), isFalse);

    final memos = OnPremRustDeskMemos.decode('');
    expect(
        memos.put(_home.tailnet, 'mac',
            const OnPremRustDeskMemo(state: OnPremRustDesk.reachable, id: '42')),
        isTrue);
    final back = OnPremRustDeskMemos.decode(memos.encode());
    expect(back.get(_home.tailnet, 'mac'),
        const OnPremRustDeskMemo(state: OnPremRustDesk.reachable, id: '42'));
    expect(back.get(_work.tailnet, 'mac'), isNull);
  });
}
