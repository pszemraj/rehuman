//! Unicode character-set helpers used by the cleaning engine.

use std::sync::OnceLock;

use icu_properties::props::{GeneralCategory, Script};
use icu_properties::{
    props, CodePointMapData, CodePointMapDataBorrowed, CodePointSetData, CodePointSetDataBorrowed,
};

static DEFAULT_IGNORABLES: OnceLock<CodePointSetDataBorrowed<'static>> = OnceLock::new();

static EMOJI_SET: OnceLock<CodePointSetDataBorrowed<'static>> = OnceLock::new();

static SCRIPT_MAP: OnceLock<CodePointMapDataBorrowed<'static, Script>> = OnceLock::new();

static GENERAL_CATEGORY_MAP: OnceLock<CodePointMapDataBorrowed<'static, GeneralCategory>> =
    OnceLock::new();

fn script(c: char) -> Script {
    SCRIPT_MAP
        .get_or_init(CodePointMapData::<Script>::new)
        .get(c)
}

fn general_category(c: char) -> GeneralCategory {
    GENERAL_CATEGORY_MAP
        .get_or_init(CodePointMapData::<GeneralCategory>::new)
        .get(c)
}

/// Latin-script characters (per the Unicode `Script` property), the candidates
/// for deunicode-based transliteration of letters: diacritics, ligatures,
/// phonetic/IPA extensions, and every Latin Extended block — without
/// maintaining a hardcoded block list.
///
/// # Returns
/// `true` when `c` has the Unicode `Latin` script property.
pub(crate) fn is_latin_script(c: char) -> bool {
    script(c) == Script::Latin
}

/// Unicode control characters (General_Category=Control).
///
/// # Returns
/// `true` when `c` has Unicode general category `Control`.
pub(crate) fn is_control_char(c: char) -> bool {
    general_category(c) == GeneralCategory::Control
}

/// Script-neutral symbols and punctuation: characters whose `General_Category`
/// is a symbol, punctuation, separator, other-number, or modifier-letter class
/// and whose `Script` is Common or Inherited. This is the transliteration
/// fallback surface — it deliberately excludes letters, marks, and digits of
/// concrete scripts (CJK, Cyrillic, Braille, ...), which keep dropping rather
/// than being romanized.
///
/// # Returns
/// `true` when `c` is a Common/Inherited character in one of the supported
/// symbol, punctuation, separator, number, or modifier-letter categories.
pub(crate) fn is_common_symbol_or_punctuation(c: char) -> bool {
    matches!(script(c), Script::Common | Script::Inherited)
        && matches!(
            general_category(c),
            GeneralCategory::MathSymbol
                | GeneralCategory::CurrencySymbol
                | GeneralCategory::ModifierSymbol
                | GeneralCategory::OtherSymbol
                | GeneralCategory::DashPunctuation
                | GeneralCategory::OpenPunctuation
                | GeneralCategory::ClosePunctuation
                | GeneralCategory::InitialPunctuation
                | GeneralCategory::FinalPunctuation
                | GeneralCategory::OtherPunctuation
                | GeneralCategory::OtherNumber
                | GeneralCategory::ModifierLetter
                | GeneralCategory::SpaceSeparator
                | GeneralCategory::LineSeparator
                | GeneralCategory::ParagraphSeparator
        )
}

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
