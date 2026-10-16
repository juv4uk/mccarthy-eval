use sens::{eval_program, ErrorKind, Session};

fn escaped(source: &str) -> String {
    source.replace('\\', "\\\\").replace('"', "\\\"")
}

fn meta_session() -> Session {
    let mut session = Session::default();
    sens::load_core_library(&mut session).expect("core bootstrap");
    sens::load_meta_evaluator_library(&mut session).expect("meta-eval bootstrap");
    session
}

fn meta_eval(source: &str) -> String {
    let mut session = meta_session();
    eval_program(
        &format!(r#"(my-eval (read "{}") (quote ()))"#, escaped(source)),
        &mut session,
    )
    .unwrap_or_else(|error| panic!("host failure while meta-evaluating {source}: {error}"))
    .value
    .to_string()
}

fn meta_program_result(program: &str) -> String {
    let mut session = meta_session();
    eval_program(
        &format!(
            r#"(cdr (my-eval-program (read-all "{}") (quote ())))"#,
            escaped(program)
        ),
        &mut session,
    )
    .unwrap_or_else(|error| panic!("host failure while meta-evaluating program {program}: {error}"))
    .value
    .to_string()
}

fn native_error_kind(source: &str) -> ErrorKind {
    let mut session = Session::default();
    eval_program(source, &mut session)
        .expect_err("reference program must fail")
        .kind
}

fn meta_error_kind(value: &str) -> Option<&str> {
    let mut tokens = value.trim_start_matches('(').split_whitespace();
    if tokens.next() == Some("error") {
        tokens.next()
    } else {
        None
    }
}

#[test]
fn current_required_meta_error_kinds_have_paired_reference_witnesses() {
    // Parse failures happen before my-eval receives data. Resource-limit and
    // division-by-zero kinds are not mechanisms owned by the current explicit
    // meta-evaluator. The required evaluator-originated correspondence set in
    // the current evidence scope is exercised below.
    let cases = [
        ("missing", ErrorKind::UnknownSymbol, "unbound-symbol"),
        ("(missing)", ErrorKind::UnknownSymbol, "unbound-symbol"),
        ("(42)", ErrorKind::Type, "not-callable"),
        ("((lambda (x) x))", ErrorKind::Arity, "arity"),
        ("(lambda (x x) x)", ErrorKind::InvalidForm, "invalid-form"),
        ("(lambda (car) car)", ErrorKind::InvalidForm, "invalid-form"),
    ];

    for (source, reference_kind, meta_kind) in cases {
        assert_eq!(native_error_kind(source), reference_kind, "source: {source}");
        assert_eq!(
            meta_error_kind(&meta_eval(source)),
            Some(meta_kind),
            "source: {source}"
        );
    }
}

#[test]
fn top_level_canon_definition_has_kind_parity_on_every_admitted_definition_surface() {
    // 0011 admits define/визначити; historical def remains compatibility-only
    // identity 1000. All three must reject Canon rebinding with the same
    // reference InvalidForm <-> meta invalid-form correspondence.
    for definition in ["define", "визначити", "def"] {
        for name in ["car", "перше", "ādi", "cond", "за-умовою", "anukrama"] {
            let source = format!("({definition} {name} 42)");
            assert_eq!(
                native_error_kind(&source),
                ErrorKind::InvalidForm,
                "reference must reject Canon rebinding: {source}"
            );
            assert_eq!(
                meta_error_kind(&meta_program_result(&source)),
                Some("invalid-form"),
                "meta evaluator must reject Canon rebinding: {source}"
            );
        }
    }
}

#[test]
fn top_level_macro_and_recursive_group_failures_keep_their_reference_kinds() {
    let macro_program = "(defmacro one (x) x) (one)";
    assert_eq!(native_error_kind(macro_program), ErrorKind::Arity);
    assert_eq!(
        meta_error_kind(&meta_program_result(macro_program)),
        Some("arity")
    );

    let malformed_group = "(def good (lambda (x) x)) (def bad (lambda (x x) x))";
    assert_eq!(native_error_kind(malformed_group), ErrorKind::InvalidForm);
    assert_eq!(
        meta_error_kind(&meta_program_result(malformed_group)),
        Some("invalid-form")
    );
}
