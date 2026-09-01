use std::sync::Mutex;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN, VIRTUAL_KEY, VK_BACK, VK_DOWN,
    VK_ESCAPE, VK_RETURN, VK_SPACE, VK_TAB, VK_UP,
};

use super::TrayCommand;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Back,
    Tab,
    Up,
    Down,
    Enter,
    Escape,
    Space,
    Other(u16),
}

impl Key {
    pub(super) fn from_vk(vk: VIRTUAL_KEY) -> Self {
        match vk {
            VK_BACK => Key::Back,
            VK_TAB => Key::Tab,
            VK_UP => Key::Up,
            VK_DOWN => Key::Down,
            VK_RETURN => Key::Enter,
            VK_ESCAPE => Key::Escape,
            VK_SPACE => Key::Space,
            other => Key::Other(other.0),
        }
    }

    fn to_vk(self) -> VIRTUAL_KEY {
        match self {
            Key::Back => VK_BACK,
            Key::Tab => VK_TAB,
            Key::Up => VK_UP,
            Key::Down => VK_DOWN,
            Key::Enter => VK_RETURN,
            Key::Escape => VK_ESCAPE,
            Key::Space => VK_SPACE,
            Key::Other(vk) => VIRTUAL_KEY(vk),
        }
    }
}

/// The modifier keys held alongside a hotkey's trigger key, expressed the way a caller
/// selects them, not the way Windows represents them on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub win: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct Hotkey {
    pub(super) modifiers: HOT_KEY_MODIFIERS,
    pub(super) vk: u32,
}

impl Hotkey {
    pub fn new(modifiers: Modifiers, key: Key) -> Self {
        let mut bits = HOT_KEY_MODIFIERS(0);
        if modifiers.ctrl {
            bits |= MOD_CONTROL;
        }
        if modifiers.shift {
            bits |= MOD_SHIFT;
        }
        if modifiers.alt {
            bits |= MOD_ALT;
        }
        if modifiers.win {
            bits |= MOD_WIN;
        }
        Self {
            modifiers: bits,
            vk: key.to_vk().0 as u32,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Message {
    Hotkey,
    TrayRightClick,
    Command(TrayCommand),
    KillFocus,
    Char(char),
    KeyDown(Key),
    Paint,
}

static PENDING_HIGH_SURROGATE: Mutex<Option<u16>> = Mutex::new(None);

pub(super) fn decode_wm_char(unit: u16) -> Option<char> {
    let mut pending = PENDING_HIGH_SURROGATE.lock().unwrap();
    decode_utf16_unit(&mut pending, unit)
}

fn decode_utf16_unit(pending: &mut Option<u16>, unit: u16) -> Option<char> {
    if let Some(high) = pending.take() {
        if (0xDC00..=0xDFFF).contains(&unit) {
            let scalar = 0x10000 + (u32::from(high) - 0xD800) * 0x400 + (u32::from(unit) - 0xDC00);
            char::from_u32(scalar)
        } else {
            None
        }
    } else if (0xD800..=0xDBFF).contains(&unit) {
        *pending = Some(unit);
        None
    } else {
        char::from_u32(u32::from(unit))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_bmp_characters_directly() {
        let mut pending = None;
        assert_eq!(decode_utf16_unit(&mut pending, 0x0041), Some('A'));
        assert_eq!(pending, None);
    }

    #[test]
    fn decodes_surrogate_pair_into_non_bmp_character() {
        let mut pending = None;
        assert_eq!(decode_utf16_unit(&mut pending, 0xD83D), None);
        assert_eq!(pending, Some(0xD83D));
        assert_eq!(decode_utf16_unit(&mut pending, 0xDE00), Some('😀'));
        assert_eq!(pending, None);
    }

    #[test]
    fn ignores_lone_low_surrogate() {
        let mut pending = None;
        assert_eq!(decode_utf16_unit(&mut pending, 0xDE00), None);
        assert_eq!(pending, None);
    }

    #[test]
    fn drops_pending_high_surrogate_on_invalid_follow_up() {
        let mut pending = None;
        assert_eq!(decode_utf16_unit(&mut pending, 0xD83D), None);
        assert_eq!(pending, Some(0xD83D));
        assert_eq!(decode_utf16_unit(&mut pending, 0x0041), None);
        assert_eq!(pending, None);
    }

    #[test]
    fn from_vk_round_trips_through_to_vk_for_named_keys() {
        for key in [
            Key::Back,
            Key::Tab,
            Key::Up,
            Key::Down,
            Key::Enter,
            Key::Escape,
            Key::Space,
        ] {
            assert_eq!(Key::from_vk(key.to_vk()), key);
        }
    }

    #[test]
    fn from_vk_falls_back_to_other_for_unmapped_codes() {
        assert_eq!(Key::from_vk(VIRTUAL_KEY(0x41)), Key::Other(0x41));
    }

    #[test]
    fn no_modifiers_produce_an_empty_mask() {
        let hotkey = Hotkey::new(Modifiers::default(), Key::Space);
        assert_eq!(hotkey.modifiers, HOT_KEY_MODIFIERS(0));
    }

    #[test]
    fn each_modifier_maps_to_its_own_bit() {
        let ctrl = Hotkey::new(
            Modifiers {
                ctrl: true,
                ..Default::default()
            },
            Key::Space,
        );
        assert_eq!(ctrl.modifiers, MOD_CONTROL);

        let shift = Hotkey::new(
            Modifiers {
                shift: true,
                ..Default::default()
            },
            Key::Space,
        );
        assert_eq!(shift.modifiers, MOD_SHIFT);

        let alt = Hotkey::new(
            Modifiers {
                alt: true,
                ..Default::default()
            },
            Key::Space,
        );
        assert_eq!(alt.modifiers, MOD_ALT);

        let win = Hotkey::new(
            Modifiers {
                win: true,
                ..Default::default()
            },
            Key::Space,
        );
        assert_eq!(win.modifiers, MOD_WIN);
    }

    #[test]
    fn modifiers_combine_into_a_single_mask() {
        let hotkey = Hotkey::new(
            Modifiers {
                ctrl: true,
                shift: true,
                ..Default::default()
            },
            Key::Space,
        );
        assert_eq!(hotkey.modifiers, MOD_CONTROL | MOD_SHIFT);
    }

    #[test]
    fn all_modifiers_combine_into_a_single_mask() {
        let hotkey = Hotkey::new(
            Modifiers {
                ctrl: true,
                shift: true,
                alt: true,
                win: true,
            },
            Key::Space,
        );
        assert_eq!(
            hotkey.modifiers,
            MOD_CONTROL | MOD_SHIFT | MOD_ALT | MOD_WIN
        );
    }

    #[test]
    fn new_carries_the_keys_virtual_key_code() {
        let hotkey = Hotkey::new(Modifiers::default(), Key::Other(0x41));
        assert_eq!(hotkey.vk, 0x41);
    }
}
