use sens::{eval_program, Session};

const MACRO_SOURCE: &str = include_str!("../../../lib/macro.lisp");

#[test]
fn exact_defmacro_already_preserves_raw_arguments() {
    let mut session = Session::default();
    let result = eval_program(
        r#"
        (00001010 first-form (a b) a)
        (first-form (00000001 ok) never-defined)
        "#,
        &mut session,
    )
    .expect("exact 00001010 must expand before evaluating the unused argument");

    assert_eq!(result.value.to_string(), "ok");
}

#[test]
#[ignore = "RED #1492: ordinary source still receives the temporary make-macro binding"]
fn ordinary_source_cannot_reach_the_macro_materialization_mechanism() {
    let session = Session::default();
    assert!(
        session.environment.get("make-macro").is_none(),
        "Closure -> Macro materialization must not be an ordinary word-call outside the 256 SENS functions"
    );
}

#[test]
#[ignore = "RED #1492: lib/macro.lisp still emits the temporary make-macro word into user macro definitions"]
fn macro_library_emits_no_make_macro_word_call() {
    assert!(
        !MACRO_SOURCE.contains("(make-macro"),
        "outer bootstrap must not call an un-SENS word as a language function"
    );
    assert!(
        !MACRO_SOURCE.contains("(00000001 make-macro)"),
        "defmacro expansion must not reconstruct an un-SENS word-call for later execution"
    );
}
