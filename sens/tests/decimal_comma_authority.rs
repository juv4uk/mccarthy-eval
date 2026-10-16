//! #220 observer for Lisp-owned decimal-comma semantic relations.
//! Rust loads and executes the witness; it does not author same/distinct truth.

use sens::{eval_program, load_core_library, Session};

#[test]
fn decimal_comma_semantics_are_owned_by_lisp_witness() {
    let mut session = Session::default();
    load_core_library(&mut session).expect("core library");
    eval_program(
        include_str!("../../../tests/fixtures/decimal-comma-authority-witness.lisp"),
        &mut session,
    )
    .expect("decimal-comma authority witness must load");

    let verdict = eval_program("(decimal-comma-authority-witness)", &mut session)
        .expect("decimal-comma authority witness must execute")
        .value
        .to_string();

    assert!(
        verdict.starts_with("(decimal-comma-authority-witness (status pass)"),
        "Lisp-owned decimal-comma witness rejected runtime semantics: {verdict}"
    );
}
