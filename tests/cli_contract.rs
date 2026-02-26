use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

static BIN_BUILD_ONCE: OnceLock<()> = OnceLock::new();

fn target_dir() -> PathBuf {
    if let Ok(dir) = env::var("CARGO_TARGET_DIR") {
        let path = PathBuf::from(dir);
        if path.is_absolute() {
            return path;
        }
        return PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target")
}

fn ensure_bins_built() {
    BIN_BUILD_ONCE.get_or_init(|| {
        let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
        let status = Command::new(cargo)
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .args(["build", "--quiet", "--bin", "rehuman", "--bin", "ishuman"])
            .status()
            .expect("failed to invoke cargo build for CLI binaries");
        assert!(status.success(), "failed to build CLI binaries for tests");
    });
}

fn bin_path(name: &str) -> PathBuf {
    let var = format!("CARGO_BIN_EXE_{name}");
    if let Ok(path) = env::var(&var) {
        return PathBuf::from(path);
    }

    ensure_bins_built();
    let mut path = target_dir();
    path.push("debug");
    path.push(if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    });
    assert!(path.exists(), "missing CLI binary at {}", path.display());
    path
}

fn run_bin(name: &str, args: &[&str], stdin_data: Option<&str>) -> Output {
    let mut cmd = Command::new(bin_path(name));
    cmd.current_dir(env!("CARGO_MANIFEST_DIR"))
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if stdin_data.is_some() {
        cmd.stdin(Stdio::piped());
    }

    let mut child = cmd.spawn().expect("failed to spawn test command");
    if let Some(data) = stdin_data {
        let mut stdin = child.stdin.take().expect("stdin was not piped");
        stdin
            .write_all(data.as_bytes())
            .expect("failed to write stdin");
    }

    child
        .wait_with_output()
        .expect("failed to wait for command output")
}

fn stderr_text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

fn stdout_text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn make_tmp_dir() -> PathBuf {
    let mut base = target_dir();
    base.push("test-tmp");
    fs::create_dir_all(&base).expect("failed to create test temp base directory");

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before unix epoch")
        .as_nanos();
    let mut dir = base;
    dir.push(format!("cli-contract-{}-{stamp}", std::process::id()));
    fs::create_dir_all(&dir).expect("failed to create test temp directory");
    dir
}

fn write_file(path: &Path, contents: &str) {
    fs::write(path, contents).unwrap_or_else(|e| panic!("failed to write {}: {e}", path.display()));
}

#[test]
fn rehuman_rejects_invalid_bool_at_parse_time() {
    let output = run_bin("rehuman", &["--keyboard-only", "maybe"], None);
    assert!(!output.status.success());
    assert!(stderr_text(&output).contains("invalid boolean value 'maybe'"));
}

#[test]
fn rehuman_rejects_keep_emoji_with_explicit_emoji_policy() {
    let output = run_bin("rehuman", &["--keep-emoji", "--emoji-policy", "drop"], None);
    assert!(!output.status.success());
    assert!(
        stderr_text(&output).contains("cannot be used with '--emoji-policy"),
        "{}",
        stderr_text(&output)
    );
}

#[test]
fn rehuman_rejects_stream_and_inplace_combination() {
    let output = run_bin("rehuman", &["--stream", "--inplace", "input.txt"], None);
    assert!(!output.status.success());
    assert!(
        stderr_text(&output).contains("cannot be used with '--inplace'"),
        "{}",
        stderr_text(&output)
    );
}

#[test]
fn rehuman_rejects_print_config_with_processing_flags() {
    let output = run_bin("rehuman", &["--print-config", "--stats"], None);
    assert!(!output.status.success());
    assert!(
        stderr_text(&output).contains("cannot be used with '--stats'"),
        "{}",
        stderr_text(&output)
    );
}

#[test]
fn rehuman_rejects_explicit_emoji_policy_without_keyboard_mode() {
    let output = run_bin(
        "rehuman",
        &["--keyboard-only", "false", "--emoji-policy", "drop"],
        Some("x😀y"),
    );
    assert!(!output.status.success());
    assert!(
        stderr_text(&output).contains("require keyboard-only mode"),
        "{}",
        stderr_text(&output)
    );
}

#[test]
fn ishuman_rejects_explicit_emoji_policy_without_keyboard_mode() {
    let output = run_bin(
        "ishuman",
        &["--keyboard-only", "false", "--keep-emoji"],
        Some("x😀y"),
    );
    assert!(!output.status.success());
    assert!(
        stderr_text(&output).contains("require keyboard-only mode"),
        "{}",
        stderr_text(&output)
    );
}

#[test]
fn config_with_unknown_key_is_rejected() {
    let dir = make_tmp_dir();
    let cfg = dir.join("config.toml");
    write_file(
        &cfg,
        r#"version = 1
[options]
keyboard_only = true
normalise_spaces = false
"#,
    );

    let output = run_bin(
        "rehuman",
        &[
            "--config",
            cfg.to_str().expect("utf8 path"),
            "--print-config",
        ],
        None,
    );
    assert!(!output.status.success());
    assert!(
        stderr_text(&output).contains("unknown field `normalise_spaces`"),
        "{}",
        stderr_text(&output)
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn stream_output_matches_buffered_output() {
    let dir = make_tmp_dir();
    let input_path = dir.join("input.txt");
    write_file(&input_path, "“Hi”—x\nSecond line 😀\n");

    let file_arg = input_path.to_str().expect("utf8 path");
    let buffered = run_bin("rehuman", &[file_arg], None);
    assert!(buffered.status.success(), "{}", stderr_text(&buffered));

    let streamed = run_bin("rehuman", &["--stream", file_arg], None);
    assert!(streamed.status.success(), "{}", stderr_text(&streamed));

    assert_eq!(stdout_text(&buffered), stdout_text(&streamed));

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn inplace_updates_file_and_is_observable() {
    let dir = make_tmp_dir();
    let input_path = dir.join("rewrite.txt");
    write_file(&input_path, "“Hi”—x\n");

    let out = run_bin(
        "rehuman",
        &["--inplace", input_path.to_str().expect("utf8 path")],
        None,
    );
    assert!(out.status.success(), "{}", stderr_text(&out));

    let updated = fs::read_to_string(&input_path).expect("failed to read updated file");
    assert_eq!(updated, "\"Hi\"-x\n");

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn inplace_noop_preserves_clean_file() {
    let dir = make_tmp_dir();
    let input_path = dir.join("clean.txt");
    let original = "clean ascii text\n";
    write_file(&input_path, original);

    let out = run_bin(
        "rehuman",
        &["--inplace", input_path.to_str().expect("utf8 path")],
        None,
    );
    assert!(out.status.success(), "{}", stderr_text(&out));

    let current = fs::read_to_string(&input_path).expect("failed to read file");
    assert_eq!(current, original);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn ishuman_exit_codes_match_cleanliness() {
    let clean = run_bin("ishuman", &[], Some("plain ascii"));
    assert_eq!(clean.status.code(), Some(0), "{}", stderr_text(&clean));

    let dirty = run_bin("ishuman", &[], Some("“quoted”"));
    assert_eq!(dirty.status.code(), Some(1), "{}", stderr_text(&dirty));
}

#[test]
fn stats_json_contract_is_consistent_between_bins() {
    let rehuman_output = run_bin("rehuman", &["--stats-json"], Some("“a”"));
    assert!(
        rehuman_output.status.success(),
        "{}",
        stderr_text(&rehuman_output)
    );

    let ishuman_output = run_bin("ishuman", &["--json"], Some("“a”"));
    assert_eq!(
        ishuman_output.status.code(),
        Some(1),
        "{}",
        stderr_text(&ishuman_output)
    );

    let rehuman_json: serde_json::Value =
        serde_json::from_str(&stderr_text(&rehuman_output)).expect("valid rehuman stats json");
    let ishuman_json: serde_json::Value =
        serde_json::from_str(&stdout_text(&ishuman_output)).expect("valid ishuman stats json");

    assert_eq!(rehuman_json, ishuman_json);
}
