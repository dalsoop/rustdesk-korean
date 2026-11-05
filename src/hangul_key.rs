//! Sends the local Korean/English (한/영) key as the peer's own 한/영 key.
//!
//! Each OS has a different 한/영 key: Windows uses VK_HANGUL (also reported by the
//! right Alt key on Korean layouts), Linux uses the Hangul key (KEY_HANGEUL or
//! keysym Hangul), and a Mac uses whatever its "Select the previous / next input
//! source" shortcuts are (F18 when Caps Lock is remapped, ⌃Space by default),
//! plus the LANG1 key, Caps Lock when "Use the Caps Lock key to switch to and
//! from ABC" is on, and 🌐 tapped alone when "Press 🌐 key to" is "Change Input
//! Source". Sent by position, the key arrives as something the peer's
//! input method ignores, or not at all (a Mac has no keycode for Hangul). The key
//! is replaced by the peer's own: F18 for a Mac (the host switches through TIS,
//! see `platform::macos_input_source`), the Hangul key for Linux, and VK_HANGUL
//! for Windows, which is sent as a legacy control key because the Windows host
//! turns map-mode keys into scancodes and only the virtual key toggles the IME.

// The chord code is used on macOS and the single-key code on Windows/Linux; both
// are compiled everywhere so their tests run on every platform.
#![allow(dead_code)]

use base::message_proto::{ControlKey, KeyEvent, KeyboardMode};
use rdev::{Event, EventType};
use std::sync::Mutex;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Os {
    Windows,
    Linux,
    MacOS,
}

#[cfg(target_os = "windows")]
const LOCAL_OS: Os = Os::Windows;
#[cfg(target_os = "linux")]
const LOCAL_OS: Os = Os::Linux;
#[cfg(target_os = "macos")]
const LOCAL_OS: Os = Os::MacOS;

const MAC_KVK_F18: u32 = 79;
const MAC_KVK_SPACE: u32 = 49;
const MAC_KVK_JIS_KANA: u32 = 104;
const MAC_KVK_CAPS_LOCK: u32 = 57;
/// The 🌐 (Fn) key, kVK_Function.
const MAC_KVK_FUNCTION: u32 = 63;
const WIN_VK_HANGUL: u32 = 0x15;
// VK_KANA shares 0x15 with VK_HANGUL; the Japanese Kana key has scancode 0x70.
const WIN_SCANCODE_KANA: u32 = 0x70;
// KEY_HANGEUL (122) plus the X11 offset of 8.
const LINUX_KEYCODE_HANGUL: u32 = 130;
const LINUX_KEYSYM_HANGUL: u32 = 0xff31;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Target {
    /// Map/translate mode keycode in the peer's code space.
    Keycode(u32),
    /// Legacy `ControlKey::Hangul`, injected as VK_HANGUL / keysym Hangul by the peer.
    LegacyHangul,
}

fn peer_os(peer: &str) -> Option<Os> {
    match peer {
        "windows" => Some(Os::Windows),
        "linux" => Some(Os::Linux),
        "macos" => Some(Os::MacOS),
        _ => None,
    }
}

/// `position_code`/`platform_code` as rdev reports them: Windows scancode/VK,
/// Linux X keycode/keysym. A Mac's keys are chords, see [`Chord`].
fn is_local_hangul_key(local: Os, position_code: u32, platform_code: u32) -> bool {
    match local {
        Os::MacOS => platform_code == MAC_KVK_F18,
        Os::Windows => platform_code == WIN_VK_HANGUL && position_code != WIN_SCANCODE_KANA,
        Os::Linux => position_code == LINUX_KEYCODE_HANGUL || platform_code == LINUX_KEYSYM_HANGUL,
    }
}

/// The key to send, or `None` to send the local key unchanged. Between Macs a
/// chord such as ⌃Space still becomes F18, which the remote Mac handles itself.
fn target(local: Os, peer: Os, mode: KeyboardMode) -> Option<Target> {
    if local == peer && local != Os::MacOS {
        return None;
    }
    if mode == KeyboardMode::Legacy {
        return Some(Target::LegacyHangul);
    }
    match peer {
        Os::MacOS => Some(Target::Keycode(MAC_KVK_F18)),
        Os::Linux => Some(Target::Keycode(LINUX_KEYCODE_HANGUL)),
        Os::Windows => Some(Target::LegacyHangul),
    }
}

fn peer_toggle_event(local: Os, peer: Os, down: bool, key_event: &KeyEvent) -> Option<KeyEvent> {
    let mode = key_event.mode.enum_value_or(KeyboardMode::Legacy);
    let mut evt = key_event.clone();
    evt.down = down;
    match target(local, peer, mode)? {
        Target::Keycode(code) => evt.set_chr(code),
        Target::LegacyHangul => {
            evt.mode = KeyboardMode::Legacy.into();
            evt.set_control_key(ControlKey::Hangul);
        }
    }
    Some(evt)
}

fn convert(
    local: Os,
    peer: &str,
    position_code: u32,
    platform_code: u32,
    down: bool,
    key_event: &KeyEvent,
) -> Option<KeyEvent> {
    if !is_local_hangul_key(local, position_code, platform_code) {
        return None;
    }
    peer_toggle_event(local, peer_os(peer)?, down, key_event)
}

/// Modifier keys of a chord. Caps Lock and Fn are not part of it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Mods {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub command: bool,
}

impl Mods {
    /// From an NSEvent modifier mask as stored in com.apple.symbolichotkeys.
    fn from_mask(mask: i64) -> Mods {
        Mods {
            shift: mask & (1 << 17) != 0,
            ctrl: mask & (1 << 18) != 0,
            alt: mask & (1 << 19) != 0,
            command: mask & (1 << 20) != 0,
        }
    }
}

/// A macOS key plus the exact modifiers that must be down with it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chord {
    pub keycode: u32,
    pub mods: Mods,
}

/// LANG1 (the 한/영 key of Korean and Japanese keyboards) always counts.
const LANG1: Chord = Chord {
    keycode: MAC_KVK_JIS_KANA,
    mods: Mods {
        ctrl: false,
        alt: false,
        shift: false,
        command: false,
    },
};

impl Chord {
    fn name(&self) -> String {
        let mut name = String::new();
        for (on, sign) in [
            (self.mods.ctrl, "⌃"),
            (self.mods.alt, "⌥"),
            (self.mods.shift, "⇧"),
            (self.mods.command, "⌘"),
        ] {
            if on {
                name.push_str(sign);
            }
        }
        let key = match self.keycode {
            MAC_KVK_SPACE => "Space".to_owned(),
            MAC_KVK_JIS_KANA => "LANG1".to_owned(),
            36 => "Return".to_owned(),
            48 => "Tab".to_owned(),
            64 => "F17".to_owned(),
            MAC_KVK_F18 => "F18".to_owned(),
            80 => "F19".to_owned(),
            90 => "F20".to_owned(),
            code => format!("key {}", code),
        };
        name + &key
    }
}

/// macOS defaults when the user never changed the shortcut: 60 "Select the
/// previous input source" = ⌃Space, 61 "Select next source in Input menu" = ⌃⌥Space.
fn default_hotkey(id: i32) -> Option<Chord> {
    let ctrl = Mods {
        ctrl: true,
        ..Default::default()
    };
    match id {
        60 => Some(Chord {
            keycode: MAC_KVK_SPACE,
            mods: ctrl,
        }),
        61 => Some(Chord {
            keycode: MAC_KVK_SPACE,
            mods: Mods { alt: true, ..ctrl },
        }),
        _ => None,
    }
}

/// One AppleSymbolicHotKeys entry: `None` when absent (macOS uses its default),
/// otherwise its `enabled` flag and `value.parameters` (character, virtual
/// keycode, modifier mask), each `None` when missing or not a number.
pub type SymbolicHotkeyEntry = Option<(Option<bool>, Option<Vec<i64>>)>;

/// The chord an enabled input-source hotkey stands for; `None` when it is off,
/// unset (keycode 65535) or malformed.
pub fn chord_from_symbolic_hotkey(id: i32, entry: SymbolicHotkeyEntry) -> Option<Chord> {
    let Some((enabled, params)) = entry else {
        return default_hotkey(id);
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
    Some(Chord {
        keycode: *keycode as u32,
        mods: Mods::from_mask(*mask),
    })
}

/// The local 한/영 chords: the enabled hotkeys, then LANG1, without duplicates.
pub fn local_chords(hotkeys: impl IntoIterator<Item = Option<Chord>>) -> Vec<Chord> {
    let mut chords = Vec::new();
    for chord in hotkeys.into_iter().flatten().chain([LANG1]) {
        if !chords.contains(&chord) {
            chords.push(chord);
        }
    }
    chords
}

/// "Press 🌐 key to: Change Input Source" as `TISGetFnUsageType` reports it.
/// The values follow the order of that menu in System Settings: 0 Do Nothing,
/// 1 Change Input Source, 2 Show Emoji & Symbols, 3 Start Dictation.
const FN_USAGE_CHANGE_INPUT_SOURCE: i32 = 1;
const FN_USAGE_KNOWN: std::ops::RangeInclusive<i32> = 0..=3;

/// This Mac's settings that make Caps Lock or 🌐 a 한/영 key; `None` when the
/// setting could not be read.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MacKeySettings {
    /// "Use the Caps Lock key to switch to and from ABC" is on.
    pub caps_lock_switch: Option<bool>,
    /// "Press 🌐 key to", the raw `TISGetFnUsageType` value.
    pub fn_usage: Option<i32>,
}

/// A Mac setting that decides whether a key is 한/영 and can fail to be read.
/// The input-source shortcuts never count as failed: a missing entry means the
/// macOS default.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MacKeySetting {
    /// "Use the Caps Lock key to switch to and from ABC".
    CapsLockSwitch,
    /// "Press 🌐 key to".
    GlobeKeyAction,
}

impl MacKeySetting {
    /// A stable id for the settings screen.
    pub fn id(&self) -> &'static str {
        match self {
            MacKeySetting::CapsLockSwitch => "caps-lock-switch",
            MacKeySetting::GlobeKeyAction => "globe-key-action",
        }
    }
}

/// Every key this Mac treats as 한/영, gathered in one place so another source
/// (such as keys the user picks) can replace the detection.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LocalHangulKeys {
    /// The enabled input-source shortcuts, then LANG1.
    pub chords: Vec<Chord>,
    /// Caps Lock switches 한/영 (each press toggles once).
    pub caps_lock: bool,
    /// 🌐 pressed and released on its own switches 한/영.
    pub globe: bool,
    /// Settings that could not be read; their key does not count as 한/영.
    pub unreadable: Vec<MacKeySetting>,
}

/// What the settings screen shows: the local 한/영 key names in order and the
/// Mac settings that could not be read.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LocalHangulKeyDisplay {
    /// Input-source shortcuts ("⌃Space", "F18"), "Caps Lock", "🌐", "LANG1".
    pub names: Vec<String>,
    pub unreadable: Vec<MacKeySetting>,
}

/// The local 한/영 keys from the input-source shortcuts and the Caps Lock and
/// 🌐 settings.
pub fn detect_local_hangul_keys(
    hotkeys: impl IntoIterator<Item = Option<Chord>>,
    settings: MacKeySettings,
) -> LocalHangulKeys {
    let mut unreadable = Vec::new();
    let caps_lock = settings.caps_lock_switch.unwrap_or_else(|| {
        unreadable.push(MacKeySetting::CapsLockSwitch);
        false
    });
    let globe = match settings.fn_usage {
        Some(usage) if FN_USAGE_KNOWN.contains(&usage) => usage == FN_USAGE_CHANGE_INPUT_SOURCE,
        _ => {
            unreadable.push(MacKeySetting::GlobeKeyAction);
            false
        }
    };
    LocalHangulKeys {
        chords: local_chords(hotkeys),
        caps_lock,
        globe,
        unreadable,
    }
}

impl LocalHangulKeys {
    /// Shortcut chords, then Caps Lock, 🌐 and LANG1, without duplicates.
    pub fn display(&self) -> LocalHangulKeyDisplay {
        let mut names: Vec<String> = Vec::new();
        let shortcuts = self.chords.iter().filter(|c| **c != LANG1).map(Chord::name);
        let others = [
            (self.caps_lock, "Caps Lock"),
            (self.globe, "🌐"),
            (true, "LANG1"),
        ]
        .into_iter()
        .filter(|(on, _)| *on)
        .map(|(_, name)| name.to_owned());
        for name in shortcuts.chain(others) {
            if !names.contains(&name) {
                names.push(name);
            }
        }
        LocalHangulKeyDisplay {
            names,
            unreadable: self.unreadable.clone(),
        }
    }
}

/// For the settings screen: reads this Mac's settings again and returns the
/// local 한/영 keys. Empty on Windows and Linux, whose 한/영 key is fixed.
pub fn local_hangul_keys_for_display() -> LocalHangulKeyDisplay {
    #[cfg(target_os = "macos")]
    {
        crate::platform::macos_input_source::load_local_hangul_chords();
        if let Some(keys) = LOCAL_KEYS.lock().unwrap().as_ref() {
            return keys.display();
        }
    }
    LocalHangulKeyDisplay::default()
}

static LOCAL_KEYS: Mutex<Option<LocalHangulKeys>> = Mutex::new(None);
static KEY_STATE: Mutex<KeyState<Event>> = Mutex::new(KeyState::new());
/// Whether the last Caps Lock event of the grab loop came with the key down.
static CAPS_LOCK_DOWN_SEEN: Mutex<bool> = Mutex::new(false);

/// Replaces the local 한/영 keys (macOS controller); logs when they change.
pub fn set_local_keys(keys: LocalHangulKeys) {
    let mut current = LOCAL_KEYS.lock().unwrap();
    if current.as_ref() != Some(&keys) {
        let display = keys.display();
        let unreadable: Vec<&str> = display.unreadable.iter().map(|s| s.id()).collect();
        hbb_common::log::info!(
            "Local 한/영 keys: {}; unreadable settings: {:?}",
            display.names.join(", "),
            unreadable
        );
    }
    *current = Some(keys);
}

fn current_local_keys() -> LocalHangulKeys {
    if let Some(keys) = LOCAL_KEYS.lock().unwrap().clone() {
        return keys;
    }
    #[cfg(target_os = "macos")]
    crate::platform::macos_input_source::load_local_hangul_chords();
    LOCAL_KEYS.lock().unwrap().clone().unwrap_or_default()
}

/// The 🌐 key while it may still be a lone tap.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Globe<E> {
    Idle,
    /// Pressed alone; the key-down is held back until it turns out to be a tap
    /// (released alone) or part of a combination.
    Pending(E),
    /// Part of a combination: its keys go to the peer as usual.
    Combo,
    /// The pending key-down was dropped (the remote window lost the keyboard);
    /// its key-up is swallowed.
    Dropped,
}

#[derive(Debug)]
struct KeyState<E> {
    /// Chord keys whose key-down was turned into a toggle.
    held: Vec<u32>,
    globe: Globe<E>,
    /// A held-back 🌐 key-down to send before the current key.
    to_send: Option<E>,
}

impl<E> KeyState<E> {
    const fn new() -> Self {
        KeyState {
            held: Vec::new(),
            globe: Globe::Idle,
            to_send: None,
        }
    }

    /// Another key or a mouse button went down while 🌐 is held: the 🌐 key-down
    /// held back so far is due.
    fn globe_combo(&mut self) -> Option<E> {
        match std::mem::replace(&mut self.globe, Globe::Idle) {
            Globe::Pending(event) => {
                self.globe = Globe::Combo;
                Some(event)
            }
            other => {
                self.globe = other;
                None
            }
        }
    }

    fn drop_pending_globe(&mut self) -> bool {
        if matches!(self.globe, Globe::Pending(_)) {
            self.globe = Globe::Dropped;
            self.to_send = None;
            return true;
        }
        false
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MacKeyAction {
    /// Not a 한/영 key: send the key as usual.
    Pass,
    /// Send nothing.
    Swallow,
    /// Send the peer's 한/영 key down or up (a chord's own key-down / key-up).
    Toggle(bool),
    /// Send the peer's 한/영 key down and up at once (Caps Lock, 🌐 tap).
    Tap,
}

/// What a local key event means for the peer when the option is on and the
/// peer has a 한/영 key. A 🌐 key-down held back until now is left in
/// `state.to_send`.
fn mac_key_action<E>(
    keys: &LocalHangulKeys,
    state: &mut KeyState<E>,
    keycode: u32,
    down: bool,
    mods: Mods,
    event: E,
) -> MacKeyAction {
    if keycode == MAC_KVK_FUNCTION && keys.globe {
        return globe_action(&mut state.globe, down, mods, event);
    }
    if down {
        if let Some(globe_down) = state.globe_combo() {
            state.to_send = Some(globe_down);
        }
    }
    if keycode == MAC_KVK_CAPS_LOCK && keys.caps_lock {
        // Caps Lock may come as a key-down or a key-up per press; the Flutter
        // key path adds a synthesized key-up to each key-down. A caller that
        // gets lone key-ups turns them into key-downs.
        return if down {
            MacKeyAction::Tap
        } else {
            MacKeyAction::Swallow
        };
    }
    match chord_action(&keys.chords, &mut state.held, keycode, down, mods) {
        ChordAction::Pass => MacKeyAction::Pass,
        ChordAction::Swallow => MacKeyAction::Swallow,
        ChordAction::Toggle(down) => MacKeyAction::Toggle(down),
    }
}

fn globe_action<E>(globe: &mut Globe<E>, down: bool, mods: Mods, event: E) -> MacKeyAction {
    let state = std::mem::replace(globe, Globe::Idle);
    let (next, action) = match (state, down) {
        (Globe::Idle | Globe::Dropped, true) if mods != Mods::default() => {
            (Globe::Combo, MacKeyAction::Pass)
        }
        (Globe::Idle | Globe::Dropped, true) => (Globe::Pending(event), MacKeyAction::Swallow),
        (Globe::Pending(held), true) => (Globe::Pending(held), MacKeyAction::Swallow),
        (Globe::Combo, true) => (Globe::Combo, MacKeyAction::Pass),
        (Globe::Pending(_), false) => (Globe::Idle, MacKeyAction::Tap),
        (Globe::Dropped, false) => (Globe::Idle, MacKeyAction::Swallow),
        (Globe::Combo | Globe::Idle, false) => (Globe::Idle, MacKeyAction::Pass),
    };
    *globe = next;
    action
}

/// Grab loop on a Mac: whether a Caps Lock event counts as a press. macOS may
/// report each press as one event (key-down or key-up by Caps Lock state) or
/// as a key-down and a key-up; `physically_down` is the key's HID state when
/// the event arrives. A key-up right after a counted key-down is the same
/// press.
fn caps_lock_press_counts(down_seen: &mut bool, physically_down: bool) -> bool {
    if physically_down {
        *down_seen = true;
        return true;
    }
    !std::mem::replace(down_seen, false)
}

/// Grab loop on a Mac: the Caps Lock event to hand to the keyboard path as a
/// key-down when Caps Lock is a local 한/영 key for `peer`, or `None` to keep
/// the grab loop's usual handling (Caps Lock is not sent, only its state).
pub fn caps_lock_from_grab(peer: &str, event: &Event, physically_down: bool) -> Option<Event> {
    let peer: String = peer
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_lowercase();
    if peer_os(&peer).is_none() || !option_enabled() || !current_local_keys().caps_lock {
        return None;
    }
    if !caps_lock_press_counts(&mut CAPS_LOCK_DOWN_SEEN.lock().unwrap(), physically_down) {
        return None;
    }
    let mut press = event.clone();
    press.event_type = EventType::KeyPress(rdev::Key::CapsLock);
    Some(press)
}

/// A 🌐 key-down held back until now that must go to the peer before the
/// current key (🌐 turned out to be part of a combination).
pub fn take_globe_to_send() -> Option<Event> {
    KEY_STATE.lock().unwrap().to_send.take()
}

/// A mouse button went down: a 🌐 held alone becomes a combination. Returns its
/// key-down to send now.
pub fn globe_combo_by_mouse() -> Option<Event> {
    KEY_STATE.lock().unwrap().globe_combo()
}

/// The remote window lost the keyboard or another session took it: a 🌐 held
/// back is forgotten and nothing is sent for it.
pub fn drop_pending_globe() {
    if KEY_STATE.lock().unwrap().drop_pending_globe() {
        hbb_common::log::info!("Pending 🌐 key dropped: the remote window let go of the keyboard");
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ChordAction {
    /// Not a chord: send the key as usual.
    Pass,
    /// A repeat of a chord key that is already down.
    Swallow,
    /// Send the peer's 한/영 key down or up.
    Toggle(bool),
}

/// `held` lists the chord keys whose key-down was turned into a toggle, so their
/// repeats are swallowed and their key-up becomes the toggle's key-up.
fn chord_action(
    chords: &[Chord],
    held: &mut Vec<u32>,
    keycode: u32,
    down: bool,
    mods: Mods,
) -> ChordAction {
    if !down {
        return match held.iter().position(|k| *k == keycode) {
            Some(i) => {
                held.remove(i);
                ChordAction::Toggle(false)
            }
            None => ChordAction::Pass,
        };
    }
    if held.contains(&keycode) {
        return ChordAction::Swallow;
    }
    if chords
        .iter()
        .any(|c| c.keycode == keycode && c.mods == mods)
    {
        held.push(keycode);
        return ChordAction::Toggle(true);
    }
    ChordAction::Pass
}

/// The key events to send instead of `event` when it is the local 한/영 key;
/// `None` leaves the event to the normal keyboard modes.
///
/// `modifier_events(down)` gives the held modifier keys as key events for the
/// peer. A Mac chord's modifiers (the Control of ⌃Space) already went to the
/// peer, so they are released around the toggle and pressed again after it: a
/// Linux input method ignores Control+Hangul, and the peer's modifier state then
/// matches the keys still held here.
pub fn to_peer_hangul_key(
    peer: &str,
    event: &Event,
    key_event: &KeyEvent,
    modifier_events: &dyn Fn(bool) -> Vec<KeyEvent>,
) -> Option<Vec<KeyEvent>> {
    let down = match event.event_type {
        EventType::KeyPress(..) => true,
        EventType::KeyRelease(..) => false,
        _ => return None,
    };
    #[cfg(target_os = "macos")]
    {
        let (alt, ctrl, shift, command) =
            crate::keyboard::client::get_modifiers_state(false, false, false, false);
        let mods = Mods {
            ctrl,
            alt,
            shift,
            command,
        };
        peer_os(peer)?;
        let enabled = option_enabled();
        if !enabled {
            return None;
        }
        let keys = current_local_keys();
        let input = KeyInput {
            keycode: event.platform_code,
            down,
            mods,
            event: event.clone(),
        };
        let mut state = KEY_STATE.lock().unwrap();
        return mac_key_events(
            peer,
            enabled,
            &keys,
            &mut state,
            input,
            key_event,
            modifier_events,
        );
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = modifier_events;
        single_key_events(peer, event, key_event, down)
    }
}

fn option_enabled() -> bool {
    use base::config::keys::OPTION_ENABLE_PEER_HANGUL_KEY;
    use hbb_common::config::LocalConfig;
    LocalConfig::get_bool_option(OPTION_ENABLE_PEER_HANGUL_KEY)
}

fn with_legacy_modifiers(mut evt: KeyEvent) -> KeyEvent {
    if evt.mode.enum_value_or(KeyboardMode::Legacy) == KeyboardMode::Legacy {
        let (alt, ctrl, shift, command) =
            crate::keyboard::client::get_modifiers_state(false, false, false, false);
        crate::keyboard::client::legacy_modifiers(&mut evt, alt, ctrl, shift, command);
    }
    evt
}

#[cfg(not(target_os = "macos"))]
fn single_key_events(
    peer: &str,
    event: &Event,
    key_event: &KeyEvent,
    down: bool,
) -> Option<Vec<KeyEvent>> {
    if !is_local_hangul_key(LOCAL_OS, event.position_code, event.platform_code) || !option_enabled()
    {
        return None;
    }
    let converted = convert(
        LOCAL_OS,
        peer,
        event.position_code,
        event.platform_code,
        down,
        key_event,
    );
    if down {
        match &converted {
            Some(evt) => hbb_common::log::info!(
                "Local 한/영 key sent to {} peer as {:?} ({:?} mode)",
                peer,
                evt.union,
                evt.mode
            ),
            None => hbb_common::log::info!("Local 한/영 key sent to {} peer unchanged", peer),
        }
    }
    Some(vec![with_legacy_modifiers(converted?)])
}

/// A local key event on a Mac as the 한/영 logic needs it.
struct KeyInput<E> {
    /// macOS virtual keycode.
    keycode: u32,
    down: bool,
    /// Modifiers held besides the key itself.
    mods: Mods,
    event: E,
}

/// The key events to send for a Mac's key event, or `None` to send it as usual:
/// always when the option is off (`enabled`) or the peer has no 한/영 key.
fn mac_key_events<E>(
    peer: &str,
    enabled: bool,
    keys: &LocalHangulKeys,
    state: &mut KeyState<E>,
    input: KeyInput<E>,
    key_event: &KeyEvent,
    modifier_events: &dyn Fn(bool) -> Vec<KeyEvent>,
) -> Option<Vec<KeyEvent>> {
    let peer_os = peer_os(peer)?;
    if !enabled {
        return None;
    }
    let KeyInput {
        keycode,
        down,
        mods,
        event,
    } = input;
    let toggles: &[bool] = match mac_key_action(keys, state, keycode, down, mods, event) {
        MacKeyAction::Pass => return None,
        MacKeyAction::Swallow => return Some(Vec::new()),
        MacKeyAction::Toggle(true) => &[true],
        MacKeyAction::Toggle(false) => &[false],
        MacKeyAction::Tap => &[true, false],
    };
    let mut toggle_events = Vec::new();
    for toggle_down in toggles {
        toggle_events.push(peer_toggle_event(
            Os::MacOS,
            peer_os,
            *toggle_down,
            key_event,
        )?);
    }
    if toggles[0] {
        let name = match keycode {
            MAC_KVK_CAPS_LOCK if keys.caps_lock => "Caps Lock".to_owned(),
            MAC_KVK_FUNCTION if keys.globe => "🌐".to_owned(),
            _ => Chord { keycode, mods }.name(),
        };
        hbb_common::log::info!(
            "Local 한/영 key {} sent to {} peer as {:?} ({:?} mode)",
            name,
            peer,
            toggle_events[0].union,
            toggle_events[0].mode
        );
    }
    // A chord's modifiers (the Control of ⌃Space) already went to the peer.
    let around = toggles[0] && mods != Mods::default();
    let mut events = Vec::new();
    if around {
        events.extend(modifier_events(false));
        events.extend(toggle_events);
        events.extend(modifier_events(true));
    } else {
        events.extend(toggle_events.into_iter().map(with_legacy_modifiers));
    }
    Some(events)
}

#[cfg(test)]
mod tests {
    use super::*;
    use base::message_proto::key_event;

    // (local OS, position code, platform code) of each OS's 한/영 key.
    const MAC_F18: (Os, u32, u32) = (Os::MacOS, 79, 79);
    const WIN_HANGUL: (Os, u32, u32) = (Os::Windows, 0x72, 0x15);
    const WIN_RALT_HANGUL: (Os, u32, u32) = (Os::Windows, 0xE038, 0x15);
    const LINUX_HANGUL: (Os, u32, u32) = (Os::Linux, 130, 0xff31);
    const LINUX_RALT_HANGUL: (Os, u32, u32) = (Os::Linux, 108, 0xff31);

    fn send(key: (Os, u32, u32), peer: &str, mode: KeyboardMode) -> Option<KeyEvent> {
        let mut key_event = KeyEvent::new();
        key_event.mode = mode.into();
        convert(key.0, peer, key.1, key.2, true, &key_event)
    }

    fn keycode(evt: Option<KeyEvent>) -> Option<(KeyboardMode, u32)> {
        let evt = evt?;
        match evt.union {
            Some(key_event::Union::Chr(code)) => Some((evt.mode.enum_value().ok()?, code)),
            _ => None,
        }
    }

    fn is_legacy_hangul(evt: Option<KeyEvent>) -> bool {
        evt.map_or(false, |evt| {
            evt.mode.enum_value() == Ok(KeyboardMode::Legacy)
                && matches!(evt.union, Some(key_event::Union::ControlKey(ck))
                    if ck.enum_value() == Ok(ControlKey::Hangul))
        })
    }

    #[test]
    fn hangul_key_becomes_the_peer_hangul_key() {
        for mode in [KeyboardMode::Map, KeyboardMode::Translate] {
            for key in [WIN_HANGUL, WIN_RALT_HANGUL, LINUX_HANGUL, LINUX_RALT_HANGUL] {
                assert_eq!(
                    keycode(send(key, "macos", mode)),
                    Some((mode, 79)),
                    "{key:?}"
                );
            }
            for key in [MAC_F18, WIN_HANGUL, WIN_RALT_HANGUL] {
                assert_eq!(
                    keycode(send(key, "linux", mode)),
                    Some((mode, 130)),
                    "{key:?}"
                );
            }
            for key in [MAC_F18, LINUX_HANGUL, LINUX_RALT_HANGUL] {
                assert!(is_legacy_hangul(send(key, "windows", mode)), "{key:?}");
            }
        }
        for (key, peer) in [
            (MAC_F18, "windows"),
            (WIN_HANGUL, "macos"),
            (LINUX_HANGUL, "macos"),
        ] {
            assert!(
                is_legacy_hangul(send(key, peer, KeyboardMode::Legacy)),
                "{key:?}"
            );
        }
    }

    #[test]
    fn same_os_and_other_keys_are_unchanged() {
        for mode in [
            KeyboardMode::Map,
            KeyboardMode::Translate,
            KeyboardMode::Legacy,
        ] {
            assert!(send(WIN_HANGUL, "windows", mode).is_none());
            assert!(send(LINUX_HANGUL, "linux", mode).is_none());
            assert!(send(MAC_F18, "android", mode).is_none());
            // Japanese Kana (VK_KANA = 0x15), macOS F19, Linux Hanja.
            assert!(send((Os::Windows, 0x70, 0x15), "macos", mode).is_none());
            assert!(send((Os::MacOS, 80, 80), "windows", mode).is_none());
            assert!(send((Os::Linux, 131, 0xff34), "macos", mode).is_none());
        }
    }

    // Without Input Monitoring the macOS client gets keys from Flutter: USB HID
    // usage -> rdev key -> macOS keycode, as in
    // `Session::_handle_key_non_flutter_simulation`. F18 (Caps Lock) must come
    // out as keycode 79, the code a hotkey chord stores.
    #[cfg(target_os = "macos")]
    #[test]
    fn flutter_f18_on_macos_is_the_local_hangul_key() {
        let key = rdev::usb_hid_key_from_code(0x6D);
        let code = rdev::code_from_key(key).map(|c| c as u32);
        assert_eq!(code, Some(MAC_KVK_F18));
        let code = MAC_KVK_F18;
        assert!(is_local_hangul_key(Os::MacOS, code, code));
        let mut key_event = KeyEvent::new();
        key_event.mode = KeyboardMode::Map.into();
        assert_eq!(
            keycode(convert(Os::MacOS, "macos", code, code, true, &key_event)),
            Some((KeyboardMode::Map, 79))
        );
        assert_eq!(
            keycode(convert(Os::MacOS, "linux", code, code, true, &key_event)),
            Some((KeyboardMode::Map, 130))
        );
        assert!(is_legacy_hangul(convert(
            Os::MacOS,
            "windows",
            code,
            code,
            true,
            &key_event
        )));
    }

    #[test]
    fn mac_to_mac_sends_f18() {
        for mode in [KeyboardMode::Map, KeyboardMode::Translate] {
            assert_eq!(keycode(send(MAC_F18, "macos", mode)), Some((mode, 79)));
        }
        assert!(is_legacy_hangul(send(
            MAC_F18,
            "macos",
            KeyboardMode::Legacy
        )));
    }

    const CTRL: Mods = Mods {
        ctrl: true,
        alt: false,
        shift: false,
        command: false,
    };
    const NONE: Mods = Mods {
        ctrl: false,
        alt: false,
        shift: false,
        command: false,
    };

    #[test]
    fn symbolic_hotkeys_become_chords() {
        let entry = |enabled, params: &[i64]| Some((enabled, Some(params.to_vec())));
        // F18 (Fn flag set by macOS for function keys) and ⌃Space.
        assert_eq!(
            chord_from_symbolic_hotkey(60, entry(Some(true), &[65535, 79, 8388608])),
            Some(Chord {
                keycode: 79,
                mods: NONE
            })
        );
        assert_eq!(
            chord_from_symbolic_hotkey(61, entry(Some(true), &[32, 49, 262144])),
            Some(Chord {
                keycode: 49,
                mods: CTRL
            })
        );
        // ⌥⌘Space, Caps Lock bit ignored.
        assert_eq!(
            chord_from_symbolic_hotkey(61, entry(Some(true), &[32, 49, 1572864 | 65536])),
            Some(Chord {
                keycode: 49,
                mods: Mods {
                    alt: true,
                    command: true,
                    ..NONE
                }
            })
        );
        // Off, unset, malformed.
        assert_eq!(
            chord_from_symbolic_hotkey(60, entry(Some(false), &[32, 49, 262144])),
            None
        );
        assert_eq!(
            chord_from_symbolic_hotkey(60, entry(None, &[32, 49, 262144])),
            None
        );
        assert_eq!(
            chord_from_symbolic_hotkey(60, entry(Some(true), &[65535, 65535, 0])),
            None
        );
        assert_eq!(
            chord_from_symbolic_hotkey(60, entry(Some(true), &[32, 49])),
            None
        );
        assert_eq!(
            chord_from_symbolic_hotkey(60, entry(Some(true), &[32, 49, 0, 0])),
            None
        );
        assert_eq!(
            chord_from_symbolic_hotkey(60, entry(Some(true), &[32, 49, -1])),
            None
        );
        assert_eq!(
            chord_from_symbolic_hotkey(60, Some((Some(true), None))),
            None
        );
        // Absent: the macOS default.
        assert_eq!(
            chord_from_symbolic_hotkey(60, None),
            Some(Chord {
                keycode: 49,
                mods: CTRL
            })
        );
    }

    #[test]
    fn lang1_is_always_a_chord_and_names_are_readable() {
        let f18 = Chord {
            keycode: 79,
            mods: NONE,
        };
        let ctrl_space = Chord {
            keycode: 49,
            mods: CTRL,
        };
        let chords = local_chords([Some(f18), Some(ctrl_space), None, Some(f18)]);
        assert_eq!(chords, vec![f18, ctrl_space, LANG1]);
        assert_eq!(local_chords([None, None]), vec![LANG1]);
        let names: Vec<String> = chords.iter().map(Chord::name).collect();
        assert_eq!(names, ["F18", "⌃Space", "LANG1"]);
    }

    #[test]
    fn chord_needs_its_exact_modifiers_and_toggles_once_per_press() {
        let chords = local_chords([Some(Chord {
            keycode: 49,
            mods: CTRL,
        })]);
        let mut held = Vec::new();
        let shift_ctrl = Mods {
            shift: true,
            ..CTRL
        };
        // Typing Space, Shift+Space and Ctrl+Shift+Space is not 한/영.
        for mods in [
            NONE,
            Mods {
                shift: true,
                ..NONE
            },
            shift_ctrl,
        ] {
            assert_eq!(
                chord_action(&chords, &mut held, 49, true, mods),
                ChordAction::Pass
            );
            assert_eq!(
                chord_action(&chords, &mut held, 49, false, mods),
                ChordAction::Pass
            );
        }
        // Ctrl with another key.
        assert_eq!(
            chord_action(&chords, &mut held, 8, true, CTRL),
            ChordAction::Pass
        );
        // Ctrl+Space, held (autorepeat), released.
        assert_eq!(
            chord_action(&chords, &mut held, 49, true, CTRL),
            ChordAction::Toggle(true)
        );
        assert_eq!(
            chord_action(&chords, &mut held, 49, true, CTRL),
            ChordAction::Swallow
        );
        assert_eq!(
            chord_action(&chords, &mut held, 49, false, CTRL),
            ChordAction::Toggle(false)
        );
        // Control let go before Space: the key-up still ends the toggle.
        assert_eq!(
            chord_action(&chords, &mut held, 49, true, CTRL),
            ChordAction::Toggle(true)
        );
        assert_eq!(
            chord_action(&chords, &mut held, 49, false, NONE),
            ChordAction::Toggle(false)
        );
        // LANG1 alone.
        assert_eq!(
            chord_action(&chords, &mut held, 104, true, NONE),
            ChordAction::Toggle(true)
        );
        assert_eq!(
            chord_action(&chords, &mut held, 104, false, NONE),
            ChordAction::Toggle(false)
        );
        assert!(held.is_empty());
    }

    #[test]
    fn chord_toggle_is_the_peer_hangul_key() {
        let mut key_event = KeyEvent::new();
        for mode in [KeyboardMode::Map, KeyboardMode::Translate] {
            key_event.mode = mode.into();
            for (peer, code) in [(Os::MacOS, 79), (Os::Linux, 130)] {
                assert_eq!(
                    keycode(peer_toggle_event(Os::MacOS, peer, true, &key_event)),
                    Some((mode, code))
                );
            }
            assert!(is_legacy_hangul(peer_toggle_event(
                Os::MacOS,
                Os::Windows,
                true,
                &key_event
            )));
        }
    }

    fn settings(caps_lock_switch: Option<bool>, fn_usage: Option<i32>) -> MacKeySettings {
        MacKeySettings {
            caps_lock_switch,
            fn_usage,
        }
    }

    const F18: Chord = Chord {
        keycode: 79,
        mods: NONE,
    };
    const CTRL_SPACE: Chord = Chord {
        keycode: 49,
        mods: CTRL,
    };

    #[test]
    fn detected_keys_follow_the_caps_lock_and_globe_settings() {
        let hotkeys = [Some(F18), Some(CTRL_SPACE)];
        let names = |s| detect_local_hangul_keys(hotkeys, s).display().names;
        // Caps Lock switching on, 🌐 changes the input source.
        let keys = detect_local_hangul_keys(hotkeys, settings(Some(true), Some(1)));
        assert!(keys.caps_lock && keys.globe && keys.unreadable.is_empty());
        assert_eq!(
            keys.display().names,
            ["F18", "⌃Space", "Caps Lock", "🌐", "LANG1"]
        );
        // Caps Lock switching off; 🌐 does nothing, shows emoji or dictation.
        for usage in [0, 2, 3] {
            let keys = detect_local_hangul_keys(hotkeys, settings(Some(false), Some(usage)));
            assert!(!keys.caps_lock && !keys.globe && keys.unreadable.is_empty());
            assert_eq!(keys.display().names, ["F18", "⌃Space", "LANG1"]);
        }
        assert_eq!(
            names(settings(Some(true), Some(2))),
            ["F18", "⌃Space", "Caps Lock", "LANG1"]
        );
        assert_eq!(
            names(settings(Some(false), Some(1))),
            ["F18", "⌃Space", "🌐", "LANG1"]
        );
        // Shortcuts off: LANG1 and the settings keys remain.
        let keys = detect_local_hangul_keys([None, None], settings(Some(true), Some(1)));
        assert_eq!(keys.display().names, ["Caps Lock", "🌐", "LANG1"]);
        // A shortcut that is LANG1 itself is listed once.
        let lang1 = detect_local_hangul_keys([Some(LANG1)], settings(Some(false), Some(0)));
        assert_eq!(lang1.display().names, ["LANG1"]);
    }

    #[test]
    fn unreadable_settings_do_not_count_and_are_reported() {
        let keys = detect_local_hangul_keys([Some(CTRL_SPACE)], settings(None, None));
        assert!(!keys.caps_lock && !keys.globe);
        assert_eq!(
            keys.unreadable,
            [MacKeySetting::CapsLockSwitch, MacKeySetting::GlobeKeyAction]
        );
        let display = keys.display();
        assert_eq!(display.names, ["⌃Space", "LANG1"]);
        assert_eq!(
            display
                .unreadable
                .iter()
                .map(|s| s.id())
                .collect::<Vec<_>>(),
            ["caps-lock-switch", "globe-key-action"]
        );
        // An 🌐 action this code does not know is not guessed.
        let keys = detect_local_hangul_keys([], settings(Some(true), Some(7)));
        assert!(keys.caps_lock && !keys.globe);
        assert_eq!(keys.unreadable, [MacKeySetting::GlobeKeyAction]);
        let keys = detect_local_hangul_keys([], settings(None, Some(1)));
        assert!(!keys.caps_lock && keys.globe);
        assert_eq!(keys.unreadable, [MacKeySetting::CapsLockSwitch]);
    }

    const CAPS: u32 = 57;
    const GLOBE: u32 = 63;
    const KEY_E: u32 = 14;
    const LEFT_ARROW: u32 = 123;
    const F1: u32 = 122;

    fn all_keys() -> LocalHangulKeys {
        detect_local_hangul_keys([Some(CTRL_SPACE)], settings(Some(true), Some(1)))
    }

    /// Feeds key events through `mac_key_events` for a Linux peer in map mode;
    /// each event is identified by its keycode in `event`. Returns, per input,
    /// `None` (sent as usual) or the keycodes sent instead, and the held-back
    /// 🌐 key-down due before it.
    struct Session {
        peer: &'static str,
        enabled: bool,
        keys: LocalHangulKeys,
        state: KeyState<u32>,
    }

    impl Session {
        fn new(peer: &'static str, keys: LocalHangulKeys) -> Self {
            Session {
                peer,
                enabled: true,
                keys,
                state: KeyState::new(),
            }
        }

        fn key(
            &mut self,
            keycode: u32,
            down: bool,
            mods: Mods,
        ) -> (Option<Vec<(bool, u32)>>, Option<u32>) {
            let mut key_event = KeyEvent::new();
            key_event.mode = KeyboardMode::Map.into();
            let input = KeyInput {
                keycode,
                down,
                mods,
                event: keycode,
            };
            let sent = mac_key_events(
                self.peer,
                self.enabled,
                &self.keys,
                &mut self.state,
                input,
                &key_event,
                &|_| Vec::new(),
            )
            .map(|events| events.iter().map(|e| (e.down, e.chr())).collect());
            (sent, self.state.to_send.take())
        }

        fn press(&mut self, keycode: u32) -> (Option<Vec<(bool, u32)>>, Option<u32>) {
            self.key(keycode, true, NONE)
        }

        fn release(&mut self, keycode: u32) -> (Option<Vec<(bool, u32)>>, Option<u32>) {
            self.key(keycode, false, NONE)
        }
    }

    // The peer's 한/영 key (Linux: keycode 130) down and up.
    fn tap() -> Option<Vec<(bool, u32)>> {
        Some(vec![(true, 130), (false, 130)])
    }
    fn nothing() -> Option<Vec<(bool, u32)>> {
        Some(Vec::new())
    }

    #[test]
    fn globe_tapped_alone_is_the_peer_hangul_key_and_fn_is_not_sent() {
        let mut s = Session::new("linux", all_keys());
        assert_eq!(s.press(GLOBE), (nothing(), None));
        assert_eq!(s.release(GLOBE), (tap(), None));
        // Repeated key-downs while held still toggle once.
        assert_eq!(s.press(GLOBE), (nothing(), None));
        assert_eq!(s.press(GLOBE), (nothing(), None));
        assert_eq!(s.release(GLOBE), (tap(), None));
        // Windows and Mac peers get their own 한/영 key.
        let mut s = Session::new("macos", all_keys());
        s.press(GLOBE);
        assert_eq!(s.release(GLOBE).0, Some(vec![(true, 79), (false, 79)]));
        let mut s = Session::new("windows", all_keys());
        s.press(GLOBE);
        let mut key_event = KeyEvent::new();
        key_event.mode = KeyboardMode::Map.into();
        let input = KeyInput {
            keycode: GLOBE,
            down: false,
            mods: NONE,
            event: GLOBE,
        };
        let sent = mac_key_events(
            "windows",
            true,
            &s.keys,
            &mut s.state,
            input,
            &key_event,
            &|_| Vec::new(),
        )
        .unwrap();
        assert_eq!(sent.len(), 2);
        assert!(sent.iter().all(|e| is_legacy_hangul(Some(e.clone()))));
        assert_eq!((sent[0].down, sent[1].down), (true, false));
    }

    #[test]
    fn globe_with_another_key_is_a_combination_without_toggle() {
        for other in [KEY_E, LEFT_ARROW, F1, 56 /* Shift */] {
            let mut s = Session::new("linux", all_keys());
            assert_eq!(s.press(GLOBE), (nothing(), None));
            // The held-back 🌐 key-down goes first, then the key as usual.
            assert_eq!(s.press(other), (None, Some(GLOBE)), "{other}");
            assert_eq!(s.release(other), (None, None));
            assert_eq!(s.press(other), (None, None));
            assert_eq!(s.release(other), (None, None));
            assert_eq!(s.release(GLOBE), (None, None));
            // The next lone tap toggles again.
            s.press(GLOBE);
            assert_eq!(s.release(GLOBE), (tap(), None));
        }
    }

    #[test]
    fn modifier_first_then_globe_is_a_combination() {
        for mods in [
            CTRL,
            Mods { alt: true, ..NONE },
            Mods {
                shift: true,
                ..NONE
            },
            Mods {
                command: true,
                ..NONE
            },
        ] {
            let mut s = Session::new("linux", all_keys());
            assert_eq!(s.key(GLOBE, true, mods), (None, None));
            assert_eq!(s.key(GLOBE, false, mods), (None, None));
            // Modifier let go before 🌐.
            assert_eq!(s.key(GLOBE, true, mods), (None, None));
            assert_eq!(s.key(GLOBE, false, NONE), (None, None));
        }
    }

    #[test]
    fn mouse_press_while_globe_is_held_is_a_combination() {
        let mut s = Session::new("linux", all_keys());
        s.press(GLOBE);
        assert_eq!(s.state.globe_combo(), Some(GLOBE));
        assert_eq!(s.state.globe_combo(), None);
        assert_eq!(s.release(GLOBE), (None, None));
    }

    #[test]
    fn dropped_globe_sends_nothing() {
        let mut s = Session::new("linux", all_keys());
        s.press(GLOBE);
        assert!(s.state.drop_pending_globe());
        assert_eq!(s.release(GLOBE), (nothing(), None));
        // Key-up lost while the keyboard was elsewhere: the next press starts over.
        s.press(GLOBE);
        assert!(s.state.drop_pending_globe());
        assert_eq!(s.press(GLOBE), (nothing(), None));
        assert_eq!(s.release(GLOBE), (tap(), None));
        // Nothing pending: nothing to drop.
        assert!(!s.state.drop_pending_globe());
    }

    #[test]
    fn caps_lock_toggles_on_every_press_and_is_not_sent() {
        let mut s = Session::new("linux", all_keys());
        assert_eq!(s.press(CAPS), (tap(), None));
        // The key-up of the Flutter key path (synthesized after each key-down).
        assert_eq!(s.release(CAPS), (nothing(), None));
        assert_eq!(s.press(CAPS), (tap(), None));
        assert_eq!(s.press(CAPS), (tap(), None));
        // Caps Lock while 🌐 is held: 🌐 is a combination, Caps Lock still toggles.
        s.press(GLOBE);
        assert_eq!(s.press(CAPS), (tap(), Some(GLOBE)));
        assert_eq!(s.release(GLOBE), (None, None));
    }

    #[test]
    fn caps_lock_press_counts_once_per_physical_press() {
        // One event per press, as key-down or key-up by Caps Lock state.
        for physically_down in [true, false] {
            let mut seen = false;
            for _ in 0..3 {
                assert!(caps_lock_press_counts(&mut seen, physically_down));
            }
        }
        // A key-down and a key-up per press: the key-up is the same press.
        let mut seen = false;
        for _ in 0..3 {
            assert!(caps_lock_press_counts(&mut seen, true));
            assert!(!caps_lock_press_counts(&mut seen, false));
        }
        // A key-up lost: the next key-down still counts.
        assert!(caps_lock_press_counts(&mut seen, true));
        assert!(caps_lock_press_counts(&mut seen, true));
    }

    #[test]
    fn caps_lock_and_globe_are_plain_keys_when_their_setting_is_off() {
        let keys = detect_local_hangul_keys([Some(CTRL_SPACE)], settings(Some(false), Some(2)));
        let mut s = Session::new("linux", keys);
        for key in [CAPS, GLOBE] {
            assert_eq!(s.press(key), (None, None));
            assert_eq!(s.release(key), (None, None));
        }
        let keys = detect_local_hangul_keys([], settings(None, None));
        let mut s = Session::new("linux", keys);
        for key in [CAPS, GLOBE] {
            assert_eq!(s.press(key), (None, None));
            assert_eq!(s.release(key), (None, None));
        }
    }

    #[test]
    fn peer_without_hangul_key_gets_the_original_keys() {
        for peer in ["android", "", "webdesktop"] {
            let mut s = Session::new(peer, all_keys());
            for key in [CAPS, GLOBE, KEY_E, 49] {
                assert_eq!(s.press(key), (None, None), "{peer} {key}");
                assert_eq!(s.release(key), (None, None), "{peer} {key}");
            }
            assert_eq!(s.key(49, true, CTRL), (None, None));
            assert_eq!(s.state.globe, Globe::Idle);
        }
    }

    #[test]
    fn option_off_changes_no_key() {
        for peer in ["linux", "windows", "macos"] {
            let mut s = Session::new(peer, all_keys());
            s.enabled = false;
            for key in [CAPS, GLOBE, KEY_E, 104] {
                assert_eq!(s.press(key), (None, None), "{peer} {key}");
                assert_eq!(s.release(key), (None, None), "{peer} {key}");
            }
            s.press(GLOBE);
            assert_eq!(s.press(KEY_E), (None, None));
            assert_eq!(s.key(49, true, CTRL), (None, None));
            assert_eq!(s.key(49, false, CTRL), (None, None));
            assert_eq!(s.state.globe, Globe::Idle);
        }
    }

    #[test]
    fn chords_still_toggle_with_caps_lock_and_globe_detection() {
        let mut s = Session::new("linux", all_keys());
        assert_eq!(s.key(49, true, CTRL).0, Some(vec![(true, 130)]));
        assert_eq!(s.key(49, true, CTRL).0, nothing());
        assert_eq!(s.key(49, false, CTRL).0, Some(vec![(false, 130)]));
        assert_eq!(s.press(104).0, Some(vec![(true, 130)]));
        assert_eq!(s.release(104).0, Some(vec![(false, 130)]));
    }

    #[test]
    fn key_up_follows_key_down() {
        let mut key_event = KeyEvent::new();
        key_event.mode = KeyboardMode::Map.into();
        let up = convert(Os::Windows, "macos", 0x72, 0x15, false, &key_event);
        assert_eq!(up.map(|e| (e.down, e.chr())), Some((false, 79)));
    }
}
