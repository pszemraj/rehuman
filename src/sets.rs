//! Unicode character-set helpers used by the cleaning engine.

use std::sync::OnceLock;

use icu_properties::{props, CodePointSetData, CodePointSetDataBorrowed};

static DEFAULT_IGNORABLES: OnceLock<CodePointSetDataBorrowed<'static>> = OnceLock::new();

static EMOJI_SET: OnceLock<CodePointSetDataBorrowed<'static>> = OnceLock::new();

/// Hidden/format-like characters defined by Default_Ignorable_Code_Point (DI).
///
/// # Returns
/// `true` when `c` should be treated as hidden/invisible.
pub fn is_hidden_char(c: char) -> bool {
    DEFAULT_IGNORABLES
        .get_or_init(CodePointSetData::new::<props::DefaultIgnorableCodePoint>)
        .contains(c)
}

/// ASCII keyboard (US) characters + whitespace controls typically produced by keyboards.
///
/// # Returns
/// `true` when `c` is accepted by keyboard-only output mode.
pub fn is_keyboard_ascii(c: char) -> bool {
    matches!(c, '\n' | '\r' | '\t') || (c.is_ascii() && !c.is_ascii_control())
}

/// Curated non-ASCII characters allowed in extended keyboard mode.
///
/// # Returns
/// `true` when `c` is accepted by the extended keyboard allowlist.
pub fn is_extended_keyboard_char(c: char) -> bool {
    matches!(
        c,
        '€' | '£' | '¥' | '¢' | '§' | '°' | '±' | '×' | '÷' | '–' | '—' | '…' | '•' | '·'
    )
}

/// Emoji detection via the Unicode `Emoji` binary property.
///
/// # Returns
/// `true` when `c` has the Unicode `Emoji` property.
pub fn is_emoji(c: char) -> bool {
    EMOJI_SET
        .get_or_init(CodePointSetData::new::<props::Emoji>)
        .contains(c)
}
