//! Korean fork (dalsoop/rustdesk-korean): update checks follow this fork's
//! GitHub releases instead of upstream RustDesk.
//!
//! Std-only on purpose: build.rs includes this file to emit `RUSTDESK_KO_RELEASE`.
#![allow(dead_code)]

pub const REPO_OWNER: &str = "dalsoop";
pub const REPO_NAME: &str = "rustdesk-korean";
/// Only the newest non-prerelease, non-draft release ("Latest").
pub const LATEST_RELEASE_API_URL: &str =
    "https://api.github.com/repos/dalsoop/rustdesk-korean/releases/latest";
pub const RELEASE_TAG_URL_PREFIX: &str = "https://github.com/dalsoop/rustdesk-korean/releases/tag/";

/// macOS releases ship a notarized .zip, not the .dmg the root updater installs.
pub const HAS_MACOS_UPDATE_DMG: bool = false;

/// Release tags look like `1.5.0-ko.11`: upstream version, then the fork release number.
pub fn parse_release_tag(tag: &str) -> Option<(Vec<u32>, u32)> {
    let (version, ko) = tag.split_once("-ko.")?;
    Some((parse_version(version)?, parse_number(ko)?))
}

/// Whether release `tag` is newer than this build's (`version`, `ko`), compared as a tuple.
/// `None` when the tag is not a fork release tag.
pub fn is_newer_release(tag: &str, version: &str, ko: u32) -> Option<bool> {
    let latest = parse_release_tag(tag)?;
    Some(latest > (parse_version(version)?, ko))
}

/// Fork release number N from the `flutter/pubspec.yaml` build number
/// (upstream build * 100 + N), as `get_ko_release()` in build.py reads it.
pub fn ko_release_from_pubspec(pubspec: &str) -> Option<u32> {
    let build = pubspec
        .lines()
        .find_map(|line| line.strip_prefix("version:")?.trim().split_once('+'))?
        .1;
    let build = parse_number(build)?;
    (build >= 100).then(|| build % 100)
}

/// This build's fork release number, emitted by build.rs; 0 when unknown.
pub fn current_ko_release() -> u32 {
    option_env!("RUSTDESK_KO_RELEASE")
        .and_then(parse_number)
        .unwrap_or(0)
}

fn parse_version(version: &str) -> Option<Vec<u32>> {
    version.split('.').map(parse_number).collect()
}

fn parse_number(s: &str) -> Option<u32> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fork_release_tags() {
        assert_eq!(parse_release_tag("1.5.0-ko.11"), Some((vec![1, 5, 0], 11)));
        assert_eq!(parse_release_tag("1.5.1-ko.1"), Some((vec![1, 5, 1], 1)));
        for tag in [
            "",
            "1.5.0",
            "1.5.0-ko.",
            "1.5.0-ko.x",
            "1.5.0-ko.+1",
            "1.5.0-ko.1-rc",
            "-ko.1",
            "v1.5.0-ko.1",
            "1.5..0-ko.1",
            "nightly",
        ] {
            assert_eq!(parse_release_tag(tag), None, "{tag}");
        }
    }

    #[test]
    fn compares_upstream_version_then_ko() {
        // Same upstream version, higher ko.
        assert_eq!(is_newer_release("1.5.0-ko.11", "1.5.0", 10), Some(true));
        // Higher upstream version wins over a lower ko.
        assert_eq!(is_newer_release("1.5.1-ko.1", "1.5.0", 10), Some(true));
        assert_eq!(is_newer_release("1.10.0-ko.1", "1.9.0", 10), Some(true));
        // Equal is not an update.
        assert_eq!(is_newer_release("1.5.0-ko.10", "1.5.0", 10), Some(false));
        // Older is not an update.
        assert_eq!(is_newer_release("1.5.0-ko.9", "1.5.0", 10), Some(false));
        assert_eq!(is_newer_release("1.4.9-ko.20", "1.5.0", 10), Some(false));
        // Not a fork tag.
        assert_eq!(is_newer_release("1.5.1", "1.5.0", 10), None);
    }

    #[test]
    fn reads_ko_release_from_pubspec_build_number() {
        let pubspec = "name: flutter_hbb\n# comment\nversion: 1.5.0+6810\nenvironment:\n";
        assert_eq!(ko_release_from_pubspec(pubspec), Some(10));
        assert_eq!(ko_release_from_pubspec("version: 1.5.1+6901\n"), Some(1));
        // An upstream build number has no ko release.
        assert_eq!(ko_release_from_pubspec("version: 1.5.0+68\n"), None);
        assert_eq!(ko_release_from_pubspec("version: 1.5.0\n"), None);
        assert_eq!(ko_release_from_pubspec("name: x\n"), None);
    }

    #[test]
    fn current_ko_release_matches_pubspec() {
        let pubspec = include_str!("../flutter/pubspec.yaml");
        assert_eq!(
            current_ko_release(),
            ko_release_from_pubspec(pubspec).unwrap_or(0)
        );
    }
}
