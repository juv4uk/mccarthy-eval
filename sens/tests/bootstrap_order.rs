use sens::{eval_program, load_core_library, Session, Value};

#[test]
fn canonical_core_loader_installs_language_owned_defmacro_before_core() {
    let mut session = Session::default();
    load_core_library(&mut session).expect("macro.my then core.my should bootstrap cleanly");

    assert!(
        matches!(session.environment.get("defmacro"), Some(Value::Macro(_))),
        "defmacro must be a language-owned macro binding after canonical bootstrap"
    );

    let value = eval_program(
        r#"
        (defmacro choose-first (left right) left)
        (choose-first (quote ok) never-defined)
        "#,
        &mut session,
    )
    .expect("language-owned defmacro should preserve unevaluated macro arguments")
    .value;
    assert_eq!(value.to_string(), "ok");
}

#[test]
fn core_macros_work_after_canonical_macro_first_bootstrap() {
    let mut session = Session::default();
    load_core_library(&mut session).expect("macro.my then core.my should bootstrap cleanly");

    let value = eval_program("(and #d1:1 42)", &mut session)
        .expect("core.my macros should be available after canonical bootstrap")
        .value;
    assert_eq!(value.to_string(), "42");

    // D3 empty list is data, not D1:NO. Explicit false must carry one bit.
    let value = eval_program("(or #d1:0 (quote fallback))", &mut session)
        .expect("core.my OR macro should be available after canonical bootstrap")
        .value;
    assert_eq!(value.to_string(), "fallback");
}
