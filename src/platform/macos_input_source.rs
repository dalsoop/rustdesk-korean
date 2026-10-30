//! Korean/English input-source switching for keys that come from a remote peer.
//!
//! macOS only runs input-source hotkeys (e.g. Caps Lock remapped to F18 and bound
//! to "Select the previous input source") for hardware events, so the synthetic
//! key RustDesk posts never switches 한/영. Keys that mean "switch Korean/English"
//! are taken out of the input stream here and switch the input source through TIS.

use base::message_proto::{key_event, ControlKey, KeyEvent, KeyboardMode};
use std::sync::Mutex;

static SWALLOWED_KEYS: Mutex<Vec<ToggleKey>> = Mutex::new(Vec::new());

/// Symbolic hotkeys "Select the previous / next input source" (F18 for 한/영).
const INPUT_SOURCE_HOTKEYS: [i32; 2] = [60, 61];

/// "Select the previous input source", the shortcut a Mac uses for 한/영.
const PREVIOUS_INPUT_SOURCE_HOTKEY: i32 = 60;

extern "C" {
    fn MacToggleKoreanInputSource() -> bool;
    fn MacPostHotKey(keycode: u16, flags: u64) -> bool;
    fn MacReadSymbolicHotKey(
        hot_key: i32,
        enabled: *mut i32,
        params: *mut i64,
        max_params: i32,
    ) -> i32;
    fn CGSIsSymbolicHotKeyEnabled(hot_key: i32) -> bool;
    fn CGSSetSymbolicHotKeyEnabled(hot_key: i32, enabled: bool) -> i32;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ToggleKey {
    /// Map/translate mode, macOS virtual keycode.
    Keycode(u32),
    /// Legacy mode, `ControlKey::Hangul`.
    Hangul,
}

/// F18 is what Caps Lock is usually remapped to for 한/영; Lang1 (kVK_JIS_Kana)
/// is where USB HID LANG1 (the Hangul key) lands on macOS. A Windows or Linux
/// Hangul key has no macOS keycode, so it only arrives in legacy mode, as
/// `ControlKey::Hangul`.
fn toggle_key(evt: &KeyEvent) -> Option<ToggleKey> {
    const KVK_F18: u32 = 79;
    const KVK_JIS_KANA: u32 = 104;
    match (evt.mode.enum_value_or(KeyboardMode::Legacy), &evt.union) {
        (KeyboardMode::Map | KeyboardMode::Translate, Some(key_event::Union::Chr(code))) => {
            let code = code & 0x0000FFFF;
            (code == KVK_F18 || code == KVK_JIS_KANA).then_some(ToggleKey::Keycode(code))
        }
        (KeyboardMode::Legacy, Some(key_event::Union::ControlKey(ck)))
            if ck.enum_value() == Ok(ControlKey::Hangul) =>
        {
            Some(ToggleKey::Hangul)
        }
        _ => None,
    }
}

/// Returns true when `evt` was consumed as a Korean/English switch and must not
/// be simulated. Must run on the main thread.
///
/// The key-up of a consumed key-down is consumed too, and key-downs repeated
/// while the key is held switch only once. When the option is off or no Korean
/// input source is enabled, the key goes through unchanged.
pub fn handle_toggle_key(evt: &KeyEvent) -> bool {
    use base::config::keys::OPTION_ENABLE_INPUT_SOURCE_TOGGLE;
    use hbb_common::config::Config;

    let Some(key) = toggle_key(evt) else {
        return false;
    };
    let mut swallowed = SWALLOWED_KEYS.lock().unwrap();
    let held = swallowed.iter().position(|k| *k == key);
    if !evt.down {
        return match held {
            Some(i) => {
                swallowed.swap_remove(i);
                true
            }
            None => false,
        };
    }
    if held.is_some() {
        return true;
    }
    if !Config::get_bool_option(OPTION_ENABLE_INPUT_SOURCE_TOGGLE) {
        return false;
    }
    if !switch_input_source() {
        hbb_common::throttled_log!(
            std::time::Duration::from_secs(60),
            warn,
            "No Korean input source to switch to; sending the key as is"
        );
        return false;
    }
    swallowed.push(key);
    hbb_common::throttled_log!(
        std::time::Duration::from_secs(1),
        info,
        "Remote 한/영 key {:?}: switched the input source",
        key
    );
    true
}

/// Switches the way this Mac's own 한/영 shortcut does. `TISSelectInputSource`
/// from this background process reaches the frontmost app only at its next
/// activation or switch, so keys typed right after 한/영 kept the old input
/// source. The shortcut posted at the HID level is handled by the frontmost
/// app in event order. TIS stays the fallback when the shortcut is off, or
/// paused because a remote window of this Mac's own RustDesk has the keyboard.
fn switch_input_source() -> bool {
    if unsafe { CGSIsSymbolicHotKeyEnabled(PREVIOUS_INPUT_SOURCE_HOTKEY) } {
        if let Some((keycode, flags)) =
            hotkey_to_post(read_symbolic_hotkey(PREVIOUS_INPUT_SOURCE_HOTKEY))
        {
            if unsafe { MacPostHotKey(keycode, flags) } {
                return true;
            }
        }
    }
    unsafe { MacToggleKoreanInputSource() }
}

/// The keycode and modifier flags to post for the "previous input source"
/// shortcut, or `None` when it is off or malformed. The modifier mask is kept
/// whole: an F key shortcut includes SecondaryFn, and the window server does
/// not match F18 without it.
fn hotkey_to_post(entry: crate::hangul_key::SymbolicHotkeyEntry) -> Option<(u16, u64)> {
    const KVK_SPACE: u16 = 49;
    const CONTROL: u64 = 1 << 18;
    let Some((enabled, params)) = entry else {
        // No entry: macOS uses its default, ⌃Space.
        return Some((KVK_SPACE, CONTROL));
    };
    if enabled != Some(true) {
        return None;
    }
    let params = params?;
    let [_, keycode, mask] = params.as_slice() else {
        return None;
    };
    if !(0..=0x7f).contains(keycode) || *mask < 0 {
        return None;
    }
    Some((*keycode as u16, *mask as u64))
}

/// Forgets held toggle keys whose key-up will never come (peer disconnected).
pub fn reset_toggle_keys() {
    SWALLOWED_KEYS.lock().unwrap().clear();
}

/// Controlling side: the input-source hotkeys are off while any remote session
/// window has the keyboard, and back on when none has.
///
/// The window server matches symbolic hotkeys before an application or event
/// tap sees the key, so F18 (Caps Lock) would switch the local input source
/// instead of reaching the remote side. `owner` is the session window (or grab
/// owner); several windows can report themselves active in any order. Only
/// hotkeys that were on are turned off, and a marker file remembers them so a
/// later launch can turn them back on if RustDesk died while they were off.
pub fn set_remote_keyboard_active(owner: u128, active: bool, reason: &str) {
    if active {
        // Read before turning the hotkeys off; the user may have changed them.
        load_local_hangul_chords();
    }
    let mut pause = HOTKEY_PAUSE.lock().unwrap();
    let HotkeyPause { owners, suspended } = &mut *pause;
    match note_owner(owners, owner, active) {
        Transition::Suspend => suspend_hotkeys(suspended, reason),
        Transition::Restore => restore_hotkeys(suspended, reason),
        Transition::Keep => {}
    }
}

/// Turns back on the hotkeys a previous RustDesk process left off (it died
/// while a remote window had the keyboard). No-op while this process has a
/// remote window active.
pub fn restore_hotkeys_left_suspended() {
    let pause = HOTKEY_PAUSE.lock().unwrap();
    if !pause.owners.is_empty() || !pause.suspended.is_empty() {
        return;
    }
    let path = marker_path();
    let Ok(text) = std::fs::read_to_string(&path) else {
        return;
    };
    for hot_key in parse_marker(&text) {
        unsafe { CGSSetSymbolicHotKeyEnabled(hot_key, true) };
    }
    if let Err(e) = std::fs::remove_file(&path) {
        hbb_common::log::warn!("Failed to remove {}: {}", path.display(), e);
    }
    hbb_common::log::info!(
        "Input-source hotkeys {:?} restored (left off by a previous run)",
        parse_marker(&text)
    );
}

/// Reads this Mac's input-source shortcuts (60, 61) from
/// com.apple.symbolichotkeys; they and LANG1 become the local 한/영 keys.
pub fn load_local_hangul_chords() {
    use crate::hangul_key::{chord_from_symbolic_hotkey, local_chords, set_local_chords};
    let chords = local_chords(
        INPUT_SOURCE_HOTKEYS
            .iter()
            .map(|id| chord_from_symbolic_hotkey(*id, read_symbolic_hotkey(*id))),
    );
    set_local_chords(chords);
}

fn read_symbolic_hotkey(hot_key: i32) -> crate::hangul_key::SymbolicHotkeyEntry {
    const MAX: usize = 8;
    let mut enabled = -1;
    let mut params = [0i64; MAX];
    let count =
        unsafe { MacReadSymbolicHotKey(hot_key, &mut enabled, params.as_mut_ptr(), MAX as i32) };
    // -3: no entry; -1: no parameters; -2: a parameter is not a number.
    if count == -3 {
        return None;
    }
    let enabled = match enabled {
        1 => Some(true),
        0 => Some(false),
        _ => None,
    };
    let params = (count >= 0).then(|| params[..(count as usize).min(MAX)].to_vec());
    Some((enabled, params))
}

struct HotkeyPause {
    owners: Vec<u128>,
    suspended: Vec<i32>,
}

static HOTKEY_PAUSE: Mutex<HotkeyPause> = Mutex::new(HotkeyPause {
    owners: Vec::new(),
    suspended: Vec::new(),
});

#[derive(Debug, PartialEq, Eq)]
enum Transition {
    Suspend,
    Restore,
    Keep,
}

fn note_owner(owners: &mut Vec<u128>, owner: u128, active: bool) -> Transition {
    let was_empty = owners.is_empty();
    if active {
        if !owners.contains(&owner) {
            owners.push(owner);
        }
    } else {
        owners.retain(|o| *o != owner);
    }
    match (was_empty, owners.is_empty()) {
        (true, false) => Transition::Suspend,
        (false, true) => Transition::Restore,
        _ => Transition::Keep,
    }
}

fn suspend_hotkeys(suspended: &mut Vec<i32>, reason: &str) {
    use base::config::keys::OPTION_ENABLE_PEER_HANGUL_KEY;
    use hbb_common::config::LocalConfig;

    if !LocalConfig::get_bool_option(OPTION_ENABLE_PEER_HANGUL_KEY) {
        return;
    }
    for hot_key in INPUT_SOURCE_HOTKEYS {
        if unsafe { CGSIsSymbolicHotKeyEnabled(hot_key) }
            && unsafe { CGSSetSymbolicHotKeyEnabled(hot_key, false) } == 0
        {
            suspended.push(hot_key);
        }
    }
    if suspended.is_empty() {
        return;
    }
    let path = marker_path();
    if let Err(e) = std::fs::write(&path, format_marker(suspended)) {
        hbb_common::log::warn!("Failed to write {}: {}", path.display(), e);
    }
    hbb_common::log::info!(
        "Input-source hotkeys {:?} suspended ({})",
        suspended,
        reason
    );
}

fn restore_hotkeys(suspended: &mut Vec<i32>, reason: &str) {
    if suspended.is_empty() {
        return;
    }
    for hot_key in suspended.iter() {
        unsafe { CGSSetSymbolicHotKeyEnabled(*hot_key, true) };
    }
    hbb_common::log::info!("Input-source hotkeys {:?} restored ({})", suspended, reason);
    suspended.clear();
    let _ = std::fs::remove_file(marker_path());
}

fn marker_path() -> std::path::PathBuf {
    hbb_common::config::Config::path("input-source-hotkeys-suspended")
}

fn format_marker(hot_keys: &[i32]) -> String {
    hot_keys
        .iter()
        .map(|k| k.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

fn parse_marker(text: &str) -> Vec<i32> {
    text.split(',')
        .filter_map(|k| k.trim().parse::<i32>().ok())
        .filter(|k| INPUT_SOURCE_HOTKEYS.contains(k))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key_event(mode: KeyboardMode, union: key_event::Union) -> KeyEvent {
        let mut evt = KeyEvent::new();
        evt.mode = mode.into();
        evt.union = Some(union);
        evt
    }

    #[test]
    fn f18_and_lang1_switch_in_map_and_translate_mode() {
        for mode in [KeyboardMode::Map, KeyboardMode::Translate] {
            for code in [79, 104] {
                let evt = key_event(mode, key_event::Union::Chr(code));
                assert_eq!(toggle_key(&evt), Some(ToggleKey::Keycode(code)));
            }
        }
    }

    #[test]
    fn hangul_control_key_switches_in_legacy_mode() {
        let evt = key_event(
            KeyboardMode::Legacy,
            key_event::Union::ControlKey(ControlKey::Hangul.into()),
        );
        assert_eq!(toggle_key(&evt), Some(ToggleKey::Hangul));
    }

    #[test]
    fn previous_input_source_shortcut_is_posted_as_stored() {
        // Caps Lock remapped to F18 and bound to "Select the previous input
        // source": F18 with SecondaryFn, which the window server requires.
        let f18 = Some((Some(true), Some(vec![65535, 79, 0x80_0000])));
        assert_eq!(hotkey_to_post(f18), Some((79, 0x80_0000)));
        // ⌃⌥Space as stored by System Settings.
        let ctrl_alt_space = Some((Some(true), Some(vec![32, 49, 0xC_0000])));
        assert_eq!(hotkey_to_post(ctrl_alt_space), Some((49, 0xC_0000)));
        // Never changed: macOS uses ⌃Space.
        assert_eq!(hotkey_to_post(None), Some((49, 0x4_0000)));
    }

    #[test]
    fn switched_off_or_unset_shortcut_falls_back_to_tis() {
        let off = Some((Some(false), Some(vec![65535, 79, 0x80_0000])));
        assert_eq!(hotkey_to_post(off), None);
        let unset = Some((Some(true), Some(vec![65535, 65535, 0])));
        assert_eq!(hotkey_to_post(unset), None);
    }

    #[test]
    fn hotkeys_stay_off_while_any_remote_window_is_active() {
        let mut owners = Vec::new();
        assert_eq!(note_owner(&mut owners, 1, true), Transition::Suspend);
        // A second window becomes active before the first one reports leaving.
        assert_eq!(note_owner(&mut owners, 2, true), Transition::Keep);
        assert_eq!(note_owner(&mut owners, 1, false), Transition::Keep);
        assert_eq!(note_owner(&mut owners, 1, false), Transition::Keep);
        assert_eq!(note_owner(&mut owners, 2, true), Transition::Keep);
        assert_eq!(note_owner(&mut owners, 2, false), Transition::Restore);
        assert_eq!(note_owner(&mut owners, 2, false), Transition::Keep);
    }

    #[test]
    fn marker_round_trips_only_input_source_hotkeys() {
        assert_eq!(parse_marker(&format_marker(&[60, 61])), vec![60, 61]);
        assert_eq!(parse_marker("60, 7,x,61\n"), vec![60, 61]);
        assert!(parse_marker("").is_empty());
    }

    #[test]
    fn other_keys_are_simulated() {
        let cases = [
            // F17, F19 and kVK_JIS_Eisu (Lang2) keep their meaning.
            key_event(KeyboardMode::Map, key_event::Union::Chr(64)),
            key_event(KeyboardMode::Map, key_event::Union::Chr(80)),
            key_event(KeyboardMode::Map, key_event::Union::Chr(102)),
            // In legacy mode a Chr is a character, not a keycode: 'O' is 79.
            key_event(KeyboardMode::Legacy, key_event::Union::Chr(79)),
            key_event(
                KeyboardMode::Legacy,
                key_event::Union::ControlKey(ControlKey::Hanja.into()),
            ),
            key_event(
                KeyboardMode::Translate,
                key_event::Union::Seq("한".to_owned()),
            ),
        ];
        for evt in cases {
            assert_eq!(toggle_key(&evt), None, "{:?}", evt);
        }
    }
}
