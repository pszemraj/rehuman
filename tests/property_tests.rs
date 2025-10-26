use icu_properties::{maps, sets, GeneralCategory};
use proptest::prelude::*;
use rehuman::{clean, is_keyboard_ascii, CleaningOptions, EmojiPolicy, TextCleaner};
use unicode_segmentation::UnicodeSegmentation;

fn sample_string() -> impl Strategy<Value = String> {
    proptest::collection::vec(any::<char>(), 0..64).prop_map(|chars| chars.into_iter().collect())
}

fn grapheme_is_rendered_emoji(grapheme: &str) -> bool {
    let chars: Vec<char> = grapheme.chars().collect();
    let emoji = sets::emoji();
    let emoji_presentation = sets::emoji_presentation();
    let extended_pictographic = sets::extended_pictographic();

    let mut has_emoji_presentation = false;
    let mut has_extended_pictographic = false;
    let mut has_emoji = false;
    let mut has_vs16 = false;
    let mut has_zwj = false;
    let mut has_keycap = false;

    for &c in &chars {
        if emoji_presentation.contains(c) {
            has_emoji_presentation = true;
        }
        if extended_pictographic.contains(c) {
            has_extended_pictographic = true;
        }
        if emoji.contains(c) {
            has_emoji = true;
        }
        match c {
            '\u{FE0F}' => has_vs16 = true,
            '\u{200D}' => has_zwj = true,
            '\u{20E3}' => has_keycap = true,
            _ => {}
        }
    }

    has_emoji_presentation
        || has_extended_pictographic
        || (has_emoji && (has_vs16 || has_zwj || has_keycap))
}

#[test]
fn dash_property_maps_to_ascii_hyphen() {
    let dash_set = sets::dash();
    for range in dash_set.iter_ranges() {
        for codepoint in range {
            let Some(ch) = char::from_u32(codepoint) else {
                continue;
            };
            let input: String = ch.into();
            let output = clean(&input);
            assert_eq!(
                output.text, "-",
                "expected dash U+{:04X} to normalize to '-'",
                codepoint
            );
        }
    }
}

#[test]
fn space_separators_collapse_to_ascii_space() {
    let gc = maps::general_category();
    let space_data = gc.get_set_for_value(GeneralCategory::SpaceSeparator);
    let space_set = space_data.as_borrowed();

    let cleaner = TextCleaner::new(CleaningOptions::default());
    for range in space_set.iter_ranges() {
        for codepoint in range {
            let Some(ch) = char::from_u32(codepoint) else {
                continue;
            };
            let input = format!("a{ch}b");
            let output = cleaner.clean(&input);
            assert_eq!(
                output.text, "a b",
                "failed to normalize U+{:04X}",
                codepoint
            );
        }
    }
}

#[test]
fn quotation_marks_normalize_to_ascii() {
    let quotation_set = sets::quotation_mark();
    let cleaner = TextCleaner::new(CleaningOptions::default());

    for range in quotation_set.iter_ranges() {
        for codepoint in range {
            let Some(ch) = char::from_u32(codepoint) else {
                continue;
            };
            if ch == '\'' || ch == '"' {
                continue;
            }
            let input = format!("x{ch}y");
            let output = cleaner.clean(&input);
            let mapped = output
                .text
                .chars()
                .nth(1)
                .expect("output should have length 3");
            assert!(
                mapped == '\'' || mapped == '"',
                "quotation U+{:04X} normalized to unexpected char {:?}",
                codepoint,
                mapped
            );
        }
    }
}

#[test]
fn keyboard_only_keeps_plain_ascii_symbols() {
    let cleaner = TextCleaner::new(CleaningOptions {
        keyboard_only: true,
        ..CleaningOptions::default()
    });
    let input = "#123 ABC xyz ~!@[](){}";
    let output = cleaner.clean(input);
    assert_eq!(output.text, input);
    assert_eq!(output.stats.emojis_dropped, 0);
    assert_eq!(output.stats.non_keyboard_removed, 0);
}

#[test]
fn keyboard_only_drops_keycap_sequences() {
    let cleaner = TextCleaner::new(CleaningOptions {
        keyboard_only: true,
        ..CleaningOptions::default()
    });
    let output = cleaner.clean("7️⃣");
    assert_eq!(output.text, "");
    assert!(output.stats.emojis_dropped >= 1);
}

#[test]
fn keyboard_only_drops_zwj_emoji() {
    let cleaner = TextCleaner::new(CleaningOptions {
        keyboard_only: true,
        ..CleaningOptions::default()
    });
    let output = cleaner.clean("👨‍👩‍👧‍👦");
    assert_eq!(output.text, "");
    assert!(output.stats.emojis_dropped >= 1);
}

fn is_allowed_hidden(c: char) -> bool {
    matches!(c, '\u{200D}' | '\u{200C}')
        || ('\u{FE00}'..='\u{FE0F}').contains(&c)
        || ('\u{E0000}'..='\u{E007F}').contains(&c)
        || ('\u{E0100}'..='\u{E01EF}').contains(&c)
}

proptest! {
    #[test]
    fn removing_hidden_characters_eliminates_default_ignorables(input in sample_string()) {
        let output = clean(&input);
        let default_ignorables = sets::default_ignorable_code_point();
        prop_assert!(
            !output.text
                .chars()
                .any(|c| default_ignorables.contains(c) && !is_allowed_hidden(c)),
            "found default ignorable code point after cleaning"
        );
    }
}

proptest! {
    #[test]
    fn keyboard_only_mode_outputs_ascii(input in sample_string()) {
        let cleaner = TextCleaner::new(CleaningOptions {
            keyboard_only: true,
            emoji_policy: EmojiPolicy::Drop,
            ..CleaningOptions::default()
        });
        let output = cleaner.clean(&input);
        prop_assert!(output.text.chars().all(is_keyboard_ascii));
        let has_rendered_emoji = UnicodeSegmentation::graphemes(output.text.as_ref(), true)
            .any(grapheme_is_rendered_emoji);
        prop_assert!(!has_rendered_emoji);
    }
}
