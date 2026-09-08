use windows::Win32::Globalization::GetACP;

/// The system's default (non-Unicode) ANSI code page, for decoding legacy
/// text that carries no encoding of its own (old `.lnk`/`.url` shortcuts
/// predating Unicode support).
pub fn system_encoding() -> &'static encoding_rs::Encoding {
    encoding_for_codepage(unsafe { GetACP() }).unwrap_or(encoding_rs::WINDOWS_1252)
}

fn encoding_for_codepage(codepage: u32) -> Option<&'static encoding_rs::Encoding> {
    Some(match codepage {
        874 => encoding_rs::WINDOWS_874,
        932 => encoding_rs::SHIFT_JIS,
        936 => encoding_rs::GBK,
        949 => encoding_rs::EUC_KR,
        950 => encoding_rs::BIG5,
        1250 => encoding_rs::WINDOWS_1250,
        1251 => encoding_rs::WINDOWS_1251,
        1252 => encoding_rs::WINDOWS_1252,
        1253 => encoding_rs::WINDOWS_1253,
        1254 => encoding_rs::WINDOWS_1254,
        1255 => encoding_rs::WINDOWS_1255,
        1256 => encoding_rs::WINDOWS_1256,
        1257 => encoding_rs::WINDOWS_1257,
        1258 => encoding_rs::WINDOWS_1258,
        _ => return None,
    })
}

/// Decodes text that may carry a BOM (UTF-8 or UTF-16), and otherwise falls
/// back to the system code page rather than assuming UTF-8 or a fixed
/// Windows-1252 code page.
pub fn decode(bytes: &[u8]) -> std::borrow::Cow<'_, str> {
    if let Some((encoding, bom_len)) = encoding_rs::Encoding::for_bom(bytes) {
        let (text, _) = encoding.decode_without_bom_handling(&bytes[bom_len..]);
        return text;
    }
    match std::str::from_utf8(bytes) {
        Ok(text) => std::borrow::Cow::Borrowed(text),
        Err(_) => {
            let (text, _, _) = system_encoding().decode(bytes);
            text
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_known_code_pages_to_their_encoding() {
        assert_eq!(encoding_for_codepage(1252), Some(encoding_rs::WINDOWS_1252));
        assert_eq!(encoding_for_codepage(936), Some(encoding_rs::GBK));
        assert_eq!(encoding_for_codepage(932), Some(encoding_rs::SHIFT_JIS));
    }

    #[test]
    fn falls_back_to_none_for_an_unknown_code_page() {
        assert_eq!(encoding_for_codepage(1), None);
    }

    #[test]
    fn decodes_utf8_bom_and_strips_it() {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice("hello".as_bytes());
        assert_eq!(decode(&bytes), "hello");
    }

    #[test]
    fn decodes_utf16le_bom() {
        let mut bytes = vec![0xFF, 0xFE];
        for unit in "hi".encode_utf16() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
        assert_eq!(decode(&bytes), "hi");
    }

    #[test]
    fn decodes_plain_ascii_as_utf8() {
        assert_eq!(decode(b"plain text"), "plain text");
    }

    #[test]
    fn falls_back_to_system_code_page_for_non_utf8_bytes() {
        // 0xE9 is 'e' with an acute accent (e) in Windows-1252, but is not
        // valid UTF-8 on its own.
        let decoded = decode(&[0xE9]);
        assert_eq!(decoded, system_encoding().decode(&[0xE9]).0);
    }
}
