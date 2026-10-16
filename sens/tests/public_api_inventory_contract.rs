use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root must resolve")
}

fn fixture_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "sens-public-api-inventory-{}-{name}.lisp",
        std::process::id()
    ))
}

#[test]
fn scanner_finds_only_top_level_defs_and_macros() {
    let fixture = fixture_path("scanner");
    let source = r#"
(def visible-fn (lambda (x) x))
(defmacro visible-macro args args)
(00001001 visible-sid (00001000 (x) x))
; (def commented-out (lambda () 'no))
"(def text-only (lambda () 'no))"
(def wrapper
  (lambda ()
    (def nested-definition (lambda () 'not-top-level))))
"#;
    fs::write(&fixture, source).expect("fixture must be writable");

    let output = Command::new("python3")
        .arg(repo_root().join("scripts/public_api_inventory.py"))
        .arg("--scan-file")
        .arg(&fixture)
        .output()
        .expect("python3 must run inventory scanner");

    let _ = fs::remove_file(&fixture);
    assert!(
        output.status.success(),
        "scanner failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("scanner output must be UTF-8");
    let names = stdout
        .lines()
        .filter_map(|line| line.split('\t').nth(2))
        .collect::<Vec<_>>();
    assert_eq!(names, vec!["visible-fn", "visible-macro", "visible-sid", "wrapper"]);
    assert!(!stdout.contains("commented-out"));
    assert!(!stdout.contains("text-only"));
    assert!(!stdout.contains("nested-definition"));
}

#[test]
fn discovery_check_allows_unreviewed_baseline() {
    let output = Command::new("python3")
        .arg(repo_root().join("scripts/public_api_inventory.py"))
        .arg("--check")
        .output()
        .expect("python3 must run inventory scanner");

    assert!(
        output.status.success(),
        "Task 2 discovery must report unreviewed definitions without failing: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("scanner output must be UTF-8");
    assert!(stdout.contains("classification: unreviewed"));
    assert!(stdout.contains("top-level definitions:"));
}
