use std::io;

use winsp_windows::window::{Hotkey, HotkeySlot, Key, Modifiers, Window};

use crate::config::{HotkeyBinding, Settings};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CaptureOutcome {
    Cancelled,
    Invalid,
    Candidate(HotkeyBinding),
}

pub(super) fn evaluate(key: Key, modifiers: Modifiers) -> CaptureOutcome {
    if key == Key::Escape {
        return CaptureOutcome::Cancelled;
    }
    if key.is_modifier() {
        return CaptureOutcome::Invalid;
    }
    if !(modifiers.ctrl || modifiers.shift || modifiers.alt || modifiers.win) {
        return CaptureOutcome::Invalid;
    }
    CaptureOutcome::Candidate(HotkeyBinding {
        ctrl: modifiers.ctrl,
        shift: modifiers.shift,
        alt: modifiers.alt,
        win: modifiers.win,
        vk: key.vk(),
    })
}

pub(super) enum CommitResult {
    Committed,
    Conflict,
    PersistFailed(io::Error),
}

fn other_slot(slot: HotkeySlot) -> HotkeySlot {
    match slot {
        HotkeySlot::Primary => HotkeySlot::Secondary,
        HotkeySlot::Secondary => HotkeySlot::Primary,
    }
}

fn to_hotkey(binding: HotkeyBinding) -> Hotkey {
    Hotkey::new(
        Modifiers {
            ctrl: binding.ctrl,
            shift: binding.shift,
            alt: binding.alt,
            win: binding.win,
        },
        Key::Other(binding.vk),
    )
}

pub(super) fn try_commit(
    window: &Window,
    settings: &mut Settings,
    active_slot: &mut HotkeySlot,
    candidate: HotkeyBinding,
) -> CommitResult {
    let trial_slot = other_slot(*active_slot);

    if !window.register_hotkey(trial_slot, to_hotkey(candidate)) {
        return CommitResult::Conflict;
    }

    let previous = settings.hotkey;
    settings.hotkey = candidate;
    if let Err(err) = settings.save() {
        settings.hotkey = previous;
        window.unregister_hotkey(trial_slot);
        return CommitResult::PersistFailed(err);
    }

    window.unregister_hotkey(*active_slot);
    *active_slot = trial_slot;
    CommitResult::Committed
}

#[cfg(test)]
mod tests {
    use super::*;

    const VK_A: u16 = 0x41;
    const VK_CONTROL: u16 = 0x11;
    const VK_SHIFT: u16 = 0x10;
    const VK_MENU: u16 = 0x12;
    const VK_LWIN: u16 = 0x5B;

    fn modifiers(ctrl: bool, shift: bool, alt: bool, win: bool) -> Modifiers {
        Modifiers {
            ctrl,
            shift,
            alt,
            win,
        }
    }

    #[test]
    fn escape_cancels_regardless_of_modifiers() {
        assert_eq!(
            evaluate(Key::Escape, modifiers(true, true, true, true)),
            CaptureOutcome::Cancelled
        );
        assert_eq!(
            evaluate(Key::Escape, Modifiers::default()),
            CaptureOutcome::Cancelled
        );
    }

    #[test]
    fn an_ordinary_key_with_no_modifiers_is_invalid() {
        assert_eq!(
            evaluate(Key::Other(VK_A), Modifiers::default()),
            CaptureOutcome::Invalid
        );
    }

    #[test]
    fn an_ordinary_key_with_one_modifier_is_a_candidate() {
        assert_eq!(
            evaluate(Key::Other(VK_A), modifiers(true, false, false, false)),
            CaptureOutcome::Candidate(HotkeyBinding {
                ctrl: true,
                shift: false,
                alt: false,
                win: false,
                vk: VK_A,
            })
        );
        assert_eq!(
            evaluate(Key::Other(VK_A), modifiers(false, false, true, false)),
            CaptureOutcome::Candidate(HotkeyBinding {
                ctrl: false,
                shift: false,
                alt: true,
                win: false,
                vk: VK_A,
            })
        );
    }

    #[test]
    fn an_ordinary_key_with_every_modifier_is_a_candidate() {
        assert_eq!(
            evaluate(Key::Other(VK_A), modifiers(true, true, true, true)),
            CaptureOutcome::Candidate(HotkeyBinding {
                ctrl: true,
                shift: true,
                alt: true,
                win: true,
                vk: VK_A,
            })
        );
    }

    #[test]
    fn a_bare_modifier_key_is_invalid_even_when_it_reports_itself_as_held() {
        assert_eq!(
            evaluate(Key::Other(VK_CONTROL), modifiers(true, false, false, false)),
            CaptureOutcome::Invalid
        );
        assert_eq!(
            evaluate(Key::Other(VK_SHIFT), modifiers(true, true, false, false)),
            CaptureOutcome::Invalid
        );
        assert_eq!(
            evaluate(Key::Other(VK_MENU), modifiers(false, false, true, false)),
            CaptureOutcome::Invalid
        );
        assert_eq!(
            evaluate(Key::Other(VK_LWIN), modifiers(false, false, false, true)),
            CaptureOutcome::Invalid
        );
    }

    #[test]
    fn a_bare_modifier_key_with_no_other_modifiers_is_invalid() {
        assert_eq!(
            evaluate(Key::Other(VK_CONTROL), Modifiers::default()),
            CaptureOutcome::Invalid
        );
    }

    #[test]
    fn other_slot_alternates_between_primary_and_secondary() {
        assert_eq!(other_slot(HotkeySlot::Primary), HotkeySlot::Secondary);
        assert_eq!(other_slot(HotkeySlot::Secondary), HotkeySlot::Primary);
    }
}
