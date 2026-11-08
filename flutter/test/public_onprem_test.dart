import 'package:flutter_hbb/common/public_onprem.dart';
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
}
