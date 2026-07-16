//! Build script generating Unicode-derived lookup tables used by `rehuman`.

use std::collections::HashSet;
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use icu_properties::props::GeneralCategory;
use icu_properties::{
    props::{Dash, QuotationMark, Script},
    CodePointMapData, CodePointSetData,
};
use phf_codegen::Map as PhfMap;
use unicode_normalization::UnicodeNormalization;

/// The irreducible Greek data: base lowercase letter -> English name. Every
/// other mapping (uppercase, accented/polytonic forms, math symbol variants)
/// is derived from these by NFKD-stripping marks down to the base letter.
/// Note: Unicode names spell U+03BB "LAMDA"; the universal English spelling
/// is "lambda", which is why these names are listed rather than parsed.
const GREEK_BASE_NAMES: &[(char, &str)] = &[
    ('\u{03B1}', "alpha"),
    ('\u{03B2}', "beta"),
    ('\u{03B3}', "gamma"),
    ('\u{03B4}', "delta"),
    ('\u{03B5}', "epsilon"),
    ('\u{03B6}', "zeta"),
    ('\u{03B7}', "eta"),
    ('\u{03B8}', "theta"),
    ('\u{03B9}', "iota"),
    ('\u{03BA}', "kappa"),
    ('\u{03BB}', "lambda"),
    ('\u{03BC}', "mu"),
    ('\u{03BD}', "nu"),
    ('\u{03BE}', "xi"),
    ('\u{03BF}', "omicron"),
    ('\u{03C0}', "pi"),
    ('\u{03C1}', "rho"),
    ('\u{03C3}', "sigma"),
    ('\u{03C4}', "tau"),
    ('\u{03C5}', "upsilon"),
    ('\u{03C6}', "phi"),
    ('\u{03C7}', "chi"),
    ('\u{03C8}', "psi"),
    ('\u{03C9}', "omega"),
];

const QUOTE_DOUBLE_OVERRIDES: &[char] = &[
    '\u{00AB}',  // LEFT-POINTING DOUBLE ANGLE QUOTATION MARK
    '\u{00BB}',  // RIGHT-POINTING DOUBLE ANGLE QUOTATION MARK
    '\u{02BA}',  // MODIFIER LETTER DOUBLE PRIME
    '\u{02EE}',  // MODIFIER LETTER DOUBLE APOSTROPHE
    '\u{201C}',  // LEFT DOUBLE QUOTATION MARK
    '\u{201D}',  // RIGHT DOUBLE QUOTATION MARK
    '\u{201E}',  // DOUBLE LOW-9 QUOTATION MARK
    '\u{201F}',  // DOUBLE HIGH-REVERSED-9 QUOTATION MARK
    '\u{2033}',  // DOUBLE PRIME
    '\u{2036}',  // REVERSED DOUBLE PRIME
    '\u{275D}',  // HEAVY DOUBLE TURNED COMMA QUOTATION MARK ORNAMENT
    '\u{275E}',  // HEAVY DOUBLE COMMA QUOTATION MARK ORNAMENT
    '\u{2760}',  // HEAVY LOW DOUBLE COMMA QUOTATION MARK ORNAMENT
    '\u{276E}',  // HEAVY LEFT-POINTING ANGLE QUOTATION MARK ORNAMENT
    '\u{276F}',  // HEAVY RIGHT-POINTING ANGLE QUOTATION MARK ORNAMENT
    '\u{1F676}', // HEAVY DOUBLE COMMA QUOTATION MARK ORNAMENT
    '\u{1F677}', // HEAVY DOUBLE TURNED COMMA QUOTATION MARK ORNAMENT
    '\u{1F678}', // HEAVY SINGLE COMMA QUOTATION MARK ORNAMENT (rendered as double)
    '\u{2E42}',  // DOUBLE LOW-REVERSED-9 QUOTATION MARK
    '\u{301D}',  // REVERSED DOUBLE PRIME QUOTATION MARK
    '\u{301E}',  // DOUBLE PRIME QUOTATION MARK
    '\u{301F}',  // LOW DOUBLE PRIME QUOTATION MARK
];

const DOUBLE_QUOTE_REPLACEMENT: &str = "'\"'";
const SINGLE_QUOTE_REPLACEMENT: &str = "'\\''";

const QUOTE_SINGLE_OVERRIDES: &[char] = &[
    '\u{0149}',  // LATIN SMALL LETTER N PRECEDED BY APOSTROPHE
    '\u{02B9}',  // MODIFIER LETTER PRIME
    '\u{02BC}',  // MODIFIER LETTER APOSTROPHE
    '\u{055A}',  // ARMENIAN APOSTROPHE
    '\u{07F4}',  // NKO HIGH TONE APOSTROPHE
    '\u{07F5}',  // NKO LOW TONE APOSTROPHE
    '\u{2018}',  // LEFT SINGLE QUOTATION MARK
    '\u{2019}',  // RIGHT SINGLE QUOTATION MARK
    '\u{201A}',  // SINGLE LOW-9 QUOTATION MARK
    '\u{201B}',  // SINGLE HIGH-REVERSED-9 QUOTATION MARK
    '\u{2032}',  // PRIME
    '\u{2034}',  // TRIPLE PRIME
    '\u{2035}',  // REVERSED PRIME
    '\u{2037}',  // REVERSED TRIPLE PRIME
    '\u{2039}',  // SINGLE LEFT-POINTING ANGLE QUOTATION MARK
    '\u{203A}',  // SINGLE RIGHT-POINTING ANGLE QUOTATION MARK
    '\u{2057}',  // QUADRUPLE PRIME
    '\u{275B}',  // HEAVY SINGLE TURNED COMMA QUOTATION MARK ORNAMENT
    '\u{275C}',  // HEAVY SINGLE COMMA QUOTATION MARK ORNAMENT
    '\u{275F}',  // HEAVY LOW SINGLE COMMA QUOTATION MARK ORNAMENT
    '\u{FF07}',  // FULLWIDTH APOSTROPHE
    '\u{E0027}', // TAG APOSTROPHE
];

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
    let dest_path = out_dir.join("generated_tables.rs");
    let mut file = File::create(&dest_path).expect("failed to create generated tables");

    let mut space_map = PhfMap::<char>::new();
    let mut dash_map = PhfMap::<char>::new();
    let mut quote_map = PhfMap::<char>::new();

    for range in CodePointMapData::<GeneralCategory>::new()
        .iter_ranges_for_value(GeneralCategory::SpaceSeparator)
    {
        for codepoint in range {
            let ch = char::from_u32(codepoint).expect("valid code point");
            if ch != ' ' {
                space_map.entry(ch, "' '");
            }
        }
    }

    for range in CodePointSetData::new::<Dash>().iter_ranges() {
        for codepoint in range {
            let ch = char::from_u32(codepoint).expect("valid code point");
            if ch != '-' {
                dash_map.entry(ch, "'-'");
            }
        }
    }

    let mut seen_quotes = HashSet::new();

    for range in CodePointSetData::new::<QuotationMark>().iter_ranges() {
        for codepoint in range {
            let ch = char::from_u32(codepoint).expect("valid code point");
            if ch == '\'' || ch == '"' {
                continue;
            }
            let mapped = if QUOTE_DOUBLE_OVERRIDES.contains(&ch) {
                DOUBLE_QUOTE_REPLACEMENT
            } else {
                SINGLE_QUOTE_REPLACEMENT
            };
            if seen_quotes.insert(ch) {
                quote_map.entry(ch, mapped);
            }
        }
    }

    for &ch in QUOTE_SINGLE_OVERRIDES {
        if ch == '\'' || ch == '"' {
            continue;
        }
        if seen_quotes.insert(ch) {
            quote_map.entry(ch, SINGLE_QUOTE_REPLACEMENT);
        }
    }

    for &ch in QUOTE_DOUBLE_OVERRIDES {
        if ch == '\'' || ch == '"' {
            continue;
        }
        if seen_quotes.insert(ch) {
            quote_map.entry(ch, DOUBLE_QUOTE_REPLACEMENT);
        }
    }

    let greek_entries = build_greek_entries();
    let mut greek_map = PhfMap::<char>::new();
    for (ch, quoted_name) in &greek_entries {
        greek_map.entry(*ch, quoted_name);
    }

    writeln!(file, "// AUTO-GENERATED FILE, DO NOT EDIT\n").unwrap();
    write_map(&mut file, "SPACE_MAP", &space_map);
    write_map(&mut file, "DASH_MAP", &dash_map);
    write_map(&mut file, "QUOTE_MAP", &quote_map);
    let greek_display = greek_map.build().to_string();
    writeln!(
        file,
        "pub static GREEK_MAP: ::phf::Map<char, &'static str> = {greek_display};\n"
    )
    .unwrap();
}

/// Map every Greek-script character and Common-script compatibility form that
/// NFKD-reduces to one of the 24 base letters onto that letter's English name
/// (capitalized for uppercase bases). Accented monotonic/polytonic forms reduce
/// via mark stripping; mathematical alphanumeric and Greek symbol variants
/// reduce via their compatibility decompositions. Characters that don't reduce
/// to a base letter (archaic letters, Coptic, standalone breathing marks) are
/// omitted and keep dropping.
fn build_greek_entries() -> Vec<(char, String)> {
    let gc = CodePointMapData::<GeneralCategory>::new();
    let mut entries = Vec::new();

    let scripts = CodePointMapData::<Script>::new();
    for script in [Script::Greek, Script::Common] {
        for range in scripts.iter_ranges_for_value(script) {
            for codepoint in range {
                let Some(ch) = char::from_u32(codepoint) else {
                    continue;
                };
                // NFKD, keep everything that is not a combining mark, then fold
                // the sigma variants (final/lunate) onto the base sigma.
                let source = ch.to_string();
                let mut bases = source
                    .nfkd()
                    .filter(|&d| gc.get(d) != GeneralCategory::NonspacingMark)
                    .map(|d| match d {
                        '\u{03C2}' | '\u{03F2}' => '\u{03C3}', // ς, ϲ -> σ
                        other => other,
                    });
                let (Some(base), None) = (bases.next(), bases.next()) else {
                    continue;
                };
                let lowered = base.to_lowercase().next().unwrap_or(base);
                let Some(&(_, name)) = GREEK_BASE_NAMES.iter().find(|&&(b, _)| b == lowered) else {
                    continue;
                };
                if base.is_uppercase() {
                    let mut capitalized = String::with_capacity(name.len());
                    capitalized.push(name.chars().next().unwrap().to_ascii_uppercase());
                    capitalized.push_str(&name[1..]);
                    entries.push((ch, format!("\"{capitalized}\"")));
                } else {
                    entries.push((ch, format!("\"{name}\"")));
                }
            }
        }
    }
    entries
}

fn write_map(file: &mut File, name: &str, map: &PhfMap<char>) {
    let display = map.build().to_string();
    writeln!(
        file,
        "pub static {name}: ::phf::Map<char, char> = {display};\n"
    )
    .unwrap();
}
