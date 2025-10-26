use icu_properties::sets;

/// Hidden/format-like characters defined by Default_Ignorable_Code_Point (DI).
pub fn is_hidden_char(c: char) -> bool {
    sets::default_ignorable_code_point().contains(c)
}

/// ASCII keyboard (US) characters + whitespace controls typically produced by keyboards.
pub fn is_keyboard_ascii(c: char) -> bool {
    // TODO: support an "extended keyboard" mode that permits a curated non-ASCII allowlist.
    matches!(c, '\n' | '\r' | '\t') || (c.is_ascii() && !c.is_ascii_control())
}

/// Emoji detection via the Unicode `Emoji` binary property.
pub fn is_emoji(c: char) -> bool {
    sets::emoji().contains(c)
}
