use winsp_windows::window::{Hotkey, HotkeySlot, Key, Modifiers, Window};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CaptureOutcome {
    Cancelled,
    Invalid,
    Candidate(Modifiers, Key),
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
    CaptureOutcome::Candidate(modifiers, key)
}

pub(super) enum CommitResult {
    Committed,
    Conflict,
    PersistFailed(String),
}

fn other_slot(slot: HotkeySlot) -> HotkeySlot {
    match slot {
        HotkeySlot::Primary => HotkeySlot::Secondary,
        HotkeySlot::Secondary => HotkeySlot::Primary,
    }
}

pub(super) fn try_commit(
    window: &Window,
    current: (Modifiers, Key),
    active_slot: &mut HotkeySlot,
    candidate: (Modifiers, Key),
    persist: impl FnOnce(Modifiers, Key) -> Result<(), String>,
) -> CommitResult {
    if candidate == current {
        return CommitResult::Committed;
    }

    let trial_slot = other_slot(*active_slot);
    let (modifiers, key) = candidate;

    if !window.register_hotkey(trial_slot, Hotkey::new(modifiers, key)) {
        return CommitResult::Conflict;
    }

    if let Err(err) = persist(modifiers, key) {
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
    const VK_B: u16 = 0x42;
    const VK_D: u16 = 0x44;
    const VK_E: u16 = 0x45;
    const VK_CONTROL: u16 = 0x11;
    const VK_SHIFT: u16 = 0x10;
    const VK_MENU: u16 = 0x12;
    const VK_LWIN: u16 = 0x5B;
    const VK_F15: u16 = 0x7E;

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
            CaptureOutcome::Candidate(modifiers(true, false, false, false), Key::Other(VK_A))
        );
        assert_eq!(
            evaluate(Key::Other(VK_A), modifiers(false, false, true, false)),
            CaptureOutcome::Candidate(modifiers(false, false, true, false), Key::Other(VK_A))
        );
    }

    #[test]
    fn an_ordinary_key_with_every_modifier_is_a_candidate() {
        assert_eq!(
            evaluate(Key::Other(VK_A), modifiers(true, true, true, true)),
            CaptureOutcome::Candidate(modifiers(true, true, true, true), Key::Other(VK_A))
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

    #[test]
    fn a_committed_hotkey_replaces_the_active_slot_and_persists() {
        let window = Window::create("WinSpTest_TryCommitCommitted", "t", 10, 10, |_, _| {})
            .expect("window creation should succeed");
        let current = (Modifiers::default(), Key::Other(VK_D));
        let mut active_slot = HotkeySlot::Primary;
        let candidate = (modifiers(true, false, false, false), Key::Other(VK_A));

        let result = try_commit(&window, current, &mut active_slot, candidate, |_, _| Ok(()));

        assert!(matches!(result, CommitResult::Committed));
        assert_eq!(active_slot, HotkeySlot::Secondary);

        window.close();
    }

    #[test]
    fn a_persist_failure_rolls_back_the_hotkey_and_the_trial_registration() {
        let window = Window::create("WinSpTest_TryCommitPersistFailed", "t", 10, 10, |_, _| {})
            .expect("window creation should succeed");
        let previous = (modifiers(false, true, false, false), Key::Other(VK_A));
        let mut active_slot = HotkeySlot::Primary;
        let candidate = (modifiers(true, false, false, false), Key::Other(VK_B));

        let result = try_commit(&window, previous, &mut active_slot, candidate, |_, _| {
            Err("disk full".to_string())
        });

        assert!(matches!(result, CommitResult::PersistFailed(_)));
        assert_eq!(active_slot, HotkeySlot::Primary);

        window.close();
    }

    #[test]
    fn a_hotkey_already_claimed_by_another_window_is_reported_as_a_conflict() {
        let blocking_window = Window::create("WinSpTest_ConflictBlocker", "t", 10, 10, |_, _| {})
            .expect("blocking window creation should succeed");
        let claimed = Hotkey::new(
            Modifiers {
                ctrl: true,
                shift: true,
                alt: false,
                win: false,
            },
            Key::Other(VK_F15),
        );
        assert!(
            blocking_window.register_hotkey(HotkeySlot::Primary, claimed),
            "setup: the blocking window should be free to claim the hotkey first"
        );

        let window = Window::create("WinSpTest_Conflict", "t", 10, 10, |_, _| {})
            .expect("window creation should succeed");
        let previous = (modifiers(false, false, true, false), Key::Other(VK_D));
        let mut active_slot = HotkeySlot::Primary;
        let candidate = (modifiers(true, true, false, false), Key::Other(VK_F15));

        let result = try_commit(&window, previous, &mut active_slot, candidate, |_, _| {
            panic!("save must not run when the hotkey registration conflicts")
        });

        assert!(matches!(result, CommitResult::Conflict));
        assert_eq!(
            active_slot,
            HotkeySlot::Primary,
            "the active slot must not change on conflict"
        );

        blocking_window.unregister_hotkey(HotkeySlot::Primary);
        blocking_window.close();
        window.close();
    }

    #[test]
    fn unchanged_hotkey_is_a_no_op() {
        let window = Window::create("WinSpTest_TryCommitUnchanged", "t", 10, 10, |_, _| {})
            .expect("window creation should succeed");
        let current_combo = (modifiers(true, false, true, false), Key::Other(VK_E));
        assert!(
            window.register_hotkey(
                HotkeySlot::Primary,
                Hotkey::new(current_combo.0, current_combo.1)
            ),
            "setup: the window should be free to claim its own current hotkey"
        );

        let mut active_slot = HotkeySlot::Primary;

        let result = try_commit(
            &window,
            current_combo,
            &mut active_slot,
            current_combo,
            |_, _| panic!("re-selecting the current hotkey must not touch disk"),
        );

        assert!(matches!(result, CommitResult::Committed));
        assert_eq!(active_slot, HotkeySlot::Primary);

        window.unregister_hotkey(HotkeySlot::Primary);
        window.close();
    }
}
