use std::collections::BTreeMap;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use rehuman::{clean, is_emoji, CleaningOptions, EmojiPolicy, TextCleaner};

fn manifest_path<P: AsRef<Path>>(rel: P) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)
}

#[test]
fn transforms_meet_reference_counts() {
    let counts_path = manifest_path("data/tranform_counts.json");
    let reader = BufReader::new(File::open(&counts_path).expect("transform counts file"));
    let counts: BTreeMap<String, usize> =
        serde_json::from_reader(reader).expect("valid transform counts json");

    for (rel_path, expected) in counts {
        let file_path = manifest_path(&rel_path);
        let input = std::fs::read_to_string(&file_path).unwrap_or_else(|err| {
            panic!("failed to read {rel_path}: {err}");
        });
        let result = clean(&input);
        let actual = result.changes_made;
        assert!(
            actual >= expected,
            "transform count for {rel_path} expected at least {expected}, got {}",
            actual
        );
    }
}

#[test]
fn keyboard_only_drops_emojis_from_goodshit() {
    let file_path = manifest_path("data/input-files/goodshit_copypasta.md");
    let input = std::fs::read_to_string(&file_path)
        .unwrap_or_else(|err| panic!("failed to read goodshit_copypasta.md: {err}"));

    let initial_emojis = input.chars().filter(|&c| is_emoji(c)).count();
    assert!(
        initial_emojis > 0,
        "fixture should contain at least one emoji character"
    );

    let cleaner = TextCleaner::new(CleaningOptions {
        keyboard_only: true,
        emoji_policy: EmojiPolicy::Drop,
        ..CleaningOptions::default()
    });
    let result = cleaner.clean(&input);

    assert_eq!(
        result.text.chars().filter(|&c| is_emoji(c)).count(),
        0,
        "output should be free of emoji codepoints"
    );
    assert!(
        result.stats.emojis_dropped >= initial_emojis,
        "expected to drop at least {initial_emojis} emoji, dropped {}",
        result.stats.emojis_dropped
    );
}
