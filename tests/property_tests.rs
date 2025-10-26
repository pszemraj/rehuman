use icu_properties::{maps, sets, GeneralCategory};
use proptest::prelude::*;
use rehuman::{clean, is_keyboard_ascii, CleaningOptions, EmojiPolicy, TextCleaner};

fn sample_string() -> impl Strategy<Value = String> {
    proptest::collection::vec(any::<char>(), 0..64).prop_map(|chars| chars.into_iter().collect())
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

proptest! {
    #[test]
    fn removing_hidden_characters_eliminates_default_ignorables(input in sample_string()) {
        let output = clean(&input);
        let default_ignorables = sets::default_ignorable_code_point();
        prop_assert!(
            !output.text.chars().any(|c| default_ignorables.contains(c)),
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
        let emoji_set = sets::emoji();
        prop_assert!(output.text.chars().all(is_keyboard_ascii));
        prop_assert!(!output.text.chars().any(|c| emoji_set.contains(c)));
    }
}
