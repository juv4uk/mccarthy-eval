use sens::{eval_program, Session};
use std::io::Write;
use std::process::{Command, Stdio};

fn translate(source: &str, from: &str, to: &str) -> String {
    let script = concat!(env!("CARGO_MANIFEST_DIR"), "/../../scripts/translate-program.py");
    let mut child = Command::new("python3")
        .args([script, "--from", from, "--to", to, "-"])
        // Force UTF-8 I/O regardless of the host console codepage (e.g.
        // cp1251 on a Ukrainian-locale Windows machine) -- WIN-PYTHON-
        // SUBPROCESS-UTF8-ENCODING (2026-09-12). Without this, Python's
        // stdout encoding follows the OS console codepage on Windows,
        // corrupting non-ASCII (Ukrainian/Sanskrit) surface names in the
        // translated output. A no-op on Linux, which already defaults to
        // UTF-8.
        .env("PYTHONIOENCODING", "utf-8")
        .env("PYTHONUTF8", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Python surface translator should start");
    child
        .stdin
        .take()
        .expect("translator stdin")
        .write_all(source.as_bytes())
        .expect("program should reach translator");
    let output = child.wait_with_output().expect("translator should finish");
    assert!(output.status.success(), "translator failed");
    String::from_utf8(output.stdout).expect("translated source should remain UTF-8")
}

#[test]
fn translated_programs_execute_with_the_same_result_on_all_table_surfaces() {
    let english = "(car (cons 'cat 'dog))";
    let ukrainian_compact = translate(english, "en", "ук");
    let ukrainian_full = translate(english, "en", "укр");
    let sanskrit = translate(english, "en", "sa");

    let mut english_session = Session::default();
    let mut ukrainian_compact_session = Session::default();
    let mut ukrainian_full_session = Session::default();
    let mut sanskrit_session = Session::default();

    let english_result = eval_program(english, &mut english_session).expect("English program");
    let ukrainian_compact_result =
        eval_program(&ukrainian_compact, &mut ukrainian_compact_session)
            .expect("compact Ukrainian program");
    let ukrainian_full_result =
        eval_program(&ukrainian_full, &mut ukrainian_full_session)
            .expect("full Ukrainian program");
    let sanskrit_result =
        eval_program(&sanskrit, &mut sanskrit_session).expect("Sanskrit program");

    assert_eq!(english_result.value.to_string(), "cat");
    assert_eq!(ukrainian_compact_result.value.to_string(), "cat");
    assert_eq!(ukrainian_full_result.value.to_string(), "cat");
    assert_eq!(sanskrit_result.value.to_string(), "cat");
}
