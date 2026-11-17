// Korean fork, RustDesk Public flavor. No dart:io here: the shared peer views
// that use it are also compiled for web.

/// True for an ID that is an IPv4 address (optionally with a port) in
/// Tailscale's CGNAT range 100.64.0.0/10.
bool isTailscaleRangeId(String id) {
  var text = id.trim();
  final colon = text.indexOf(':');
  if (colon >= 0 && colon == text.lastIndexOf(':')) {
    text = text.substring(0, colon);
  }
  final parts = text.split('.');
  if (parts.length != 4) return false;
  final bytes = <int>[];
  for (final p in parts) {
    if (!RegExp(r'^\d{1,3}$').hasMatch(p)) return false;
    final b = int.parse(p);
    if (b > 255) return false;
    bytes.add(b);
  }
  return bytes[0] == 100 && (bytes[1] & 0xC0) == 64;
}
