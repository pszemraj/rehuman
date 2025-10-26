// Character classification helpers kept small & explicit.

/// Hidden/format-like characters that should be removed by default.
pub fn is_hidden_char(c: char) -> bool {
    matches!(
        c,
        // Zero width / format
        '\u{180E}' | // Mongolian vowel separator
        '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{FEFF}' | '\u{00AD}' |
        // Bidi marks & isolates
        '\u{200E}' | '\u{200F}' |
        '\u{202A}' | '\u{202B}' | '\u{202C}' | '\u{202D}' | '\u{202E}' |
        '\u{2066}' | '\u{2067}' | '\u{2068}' | '\u{2069}' |
        // Other 'invisible' format
        '\u{2060}' | '\u{2061}' | '\u{2062}' | '\u{2063}' | '\u{2064}' |
        // Interlinear annotation
        '\u{FFF9}' | '\u{FFFA}' | '\u{FFFB}'
    )
}

/// ASCII keyboard (US) characters + space/newline/tab.
pub fn is_keyboard_ascii(c: char) -> bool {
    if c.is_ascii() {
        // permit printable ASCII and space, and a few whitespace controls
        return c.is_ascii_graphic() || c == ' ' || c == '\n' || c == '\r' || c == '\t';
    }
    false
}

/// Heuristic emoji check via major ranges. Not exhaustive, but fast & practical.
pub fn is_emoji(c: char) -> bool {
    let code = c as u32;
    matches!(code,
        0x1F600..=0x1F64F |  // Emoticons
        0x1F300..=0x1F5FF |  // Misc symbols & pictographs
        0x1F680..=0x1F6FF |  // Transport & map
        0x1F900..=0x1F9FF |  // Supplemental symbols & pictographs
        0x1FA70..=0x1FAFF |  // Symbols & pictographs Extended-A
        0x2600..=0x26FF   |  // Misc symbols
        0x2700..=0x27BF   |  // Dingbats
        0xFE0F | 0xFE0E       // Variation selectors
    )
}
