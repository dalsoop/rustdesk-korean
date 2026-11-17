// Korean fork, RustDesk Public flavor: RustDesk's public server takes only
// third-party sign-ins (Google, GitHub, ...) and refuses connections made
// before one.

/// True when [apiServer] is RustDesk's own server (admin.rustdesk.com), by the
/// same host test as `is_public` in src/common.rs.
bool isRustDeskPublicApiServer(String apiServer) {
  final url = apiServer.contains('://') ? apiServer : 'http://$apiServer';
  var host = Uri.tryParse(url)?.host.toLowerCase() ?? '';
  if (host.endsWith('.')) host = host.substring(0, host.length - 1);
  return host == 'rustdesk.com' || host.endsWith('.rustdesk.com');
}

/// True for the public server's refusal of a connection from a client that
/// is not signed in.
bool isPublicLoginRequiredError(String text) {
  final lower = text.toLowerCase();
  return lower.contains('log in before connecting') ||
      lower.contains('login required for public server');
}
