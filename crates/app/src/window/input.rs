use std::sync::Mutex;

pub(super) static PENDING_HIGH_SURROGATE: Mutex<Option<u16>> = Mutex::new(None);

pub(super) fn decode_utf16_unit(pending: &mut Option<u16>, unit: u16) -> Option<char> {
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
}
