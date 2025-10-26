use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use icu_properties::{maps, sets, GeneralCategory};
use phf_codegen::Map as PhfMap;
use unicode_names2::name;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
    let dest_path = out_dir.join("generated_tables.rs");
    let mut file = File::create(&dest_path).expect("failed to create generated tables");

    let mut space_map = PhfMap::<char>::new();
    let mut dash_map = PhfMap::<char>::new();
    let mut quote_map = PhfMap::<char>::new();

    let general_category = maps::general_category();
    let space_separator_data = general_category.get_set_for_value(GeneralCategory::SpaceSeparator);
    let space_separators = space_separator_data.as_borrowed();
    let dash_chars = sets::dash();
    let quotation_marks = sets::quotation_mark();

    for codepoint in 0u32..=0x10FFFF {
        let Some(ch) = char::from_u32(codepoint) else {
            continue;
        };

        if ch != ' ' && space_separators.contains(ch) {
            space_map.entry(ch, "' '");
            continue;
        }

        if ch != '-' && dash_chars.contains(ch) {
            dash_map.entry(ch, "'-'");
            continue;
        }

        if ch == '\u{2212}' {
            dash_map.entry(ch, "'-'");
            continue;
        }

        if ch == '\'' || ch == '"' {
            continue;
        }

        if let Some(mapped) = map_quote(ch) {
            if quotation_marks.contains(ch) || mapped == '"' || mapped == '\'' {
                match mapped {
                    '"' => {
                        quote_map.entry(ch, "'\"'");
                    }
                    '\'' => {
                        quote_map.entry(ch, "'\\''");
                    }
                    _ => {}
                }
            }
        }
    }

    writeln!(file, "// AUTO-GENERATED FILE, DO NOT EDIT\n").unwrap();
    write_map(&mut file, "SPACE_MAP", &space_map);
    write_map(&mut file, "DASH_MAP", &dash_map);
    write_map(&mut file, "QUOTE_MAP", &quote_map);
}

fn map_quote(ch: char) -> Option<char> {
    let Some(name) = name(ch).map(|n| n.to_string()) else {
        return None;
    };
    classify_quote(&name)
}

fn classify_quote(name: &str) -> Option<char> {
    let upper = name.to_ascii_uppercase();
    if !(upper.contains("QUOTE")
        || upper.contains("QUOTATION MARK")
        || upper.contains("APOSTROPHE")
        || upper.contains("PRIME"))
    {
        return None;
    }

    let is_double = upper.contains("DOUBLE")
        || upper.contains("GUILLEMET")
        || upper.contains("DOUBLE PRIME")
        || upper.contains("DOUBLE ACUTE")
        || upper.contains("REVERSING QUOTATION MARK")
        || (upper.contains("ANGLE QUOTATION MARK") && !upper.contains("SINGLE"));

    if is_double {
        return Some('"');
    }

    let is_single = upper.contains("SINGLE")
        || upper.contains("APOSTROPHE")
        || (upper.contains("PRIME") && !upper.contains("DOUBLE"))
        || (upper.contains("ACUTE ACCENT") && !upper.contains("DOUBLE"))
        || upper.contains("TICK")
        || upper.contains("MINUTE")
        || upper.contains("SECOND");

    if is_single {
        Some('\'')
    } else {
        None
    }
}

fn write_map(file: &mut File, name: &str, map: &PhfMap<char>) {
    let display = map.build().to_string();
    writeln!(
        file,
        "pub static {name}: ::phf::Map<char, char> = {display};\n"
    )
    .unwrap();
}
