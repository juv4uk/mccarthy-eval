use sens::{eval_program, ErrorKind, Session};

fn escaped(source: &str) -> String {
    source.replace('\\', "\\\\").replace('"', "\\\"")
}

fn meta_session() -> Session {
    let mut session = Session::default();
    eval_program(include_str!("../../../lib/core.lisp"), &mut session).expect("core bootstrap");
    sens::load_meta_evaluator_library(&mut session)
        .expect("meta-eval bootstrap");
    session
}

fn meta_eval_program(program: &str, probe: &str) -> String {
    let mut session = meta_session();
    let source = format!(
        r#"(let ((loaded (my-eval-program (read-all "{}") (quote ()))))
             (my-eval (read "{}") (car loaded)))"#,
        escaped(program),
        escaped(probe),
    );
    eval_program(&source, &mut session)
        .unwrap_or_else(|error| panic!("host failure while probing meta program: {error}"))
        .value
        .to_string()
}

fn native_value(program: &str, probe: &str) -> String {
    let mut session = Session::default();
    eval_program(&format!("{program}\n{probe}"), &mut session)
        .unwrap_or_else(|error| panic!("reference evaluator failed: {error}"))
        .value
        .to_string()
}

fn native_error_kind(program: &str, probe: &str) -> ErrorKind {
    let mut session = Session::default();
    eval_program(&format!("{program}\n{probe}"), &mut session)
        .expect_err("reference program must fail")
        .kind
}

#[test]
fn later_binding_in_the_same_definition_frame_is_visible() {
    let program = r#"
(def f (lambda () g))
(def g 42)
"#;

    assert_eq!(native_value(program, "(f)"), "42");
    assert_eq!(meta_eval_program(program, "(f)"), "42");
}

#[test]
fn later_replacement_in_the_same_definition_frame_is_visible() {
    let program = r#"
(def g 1)
(def f (lambda () g))
(def g 2)
"#;

    assert_eq!(native_value(program, "(f)"), "2");
    assert_eq!(meta_eval_program(program, "(f)"), "2");
}

#[test]
fn one_way_later_lambda_binding_is_visible_without_inventing_an_scc() {
    let program = r#"
(def f (lambda (x) (g x)))
(def g (lambda (x) (+ x 1)))
"#;

    assert_eq!(native_value(program, "(f 41)"), "42");
    assert_eq!(meta_eval_program(program, "(f 41)"), "42");

    let f = meta_eval_program(program, "f");
    assert!(
        f.starts_with("(recursive-closure f "),
        "one-way dependency must remain outside a recursive SCC, got {f}"
    );
}

#[test]
fn call_before_the_later_binding_remains_unresolved() {
    // Use operator-position lookup so this witness stays isolated from the
    // separately documented bare-atom UnknownSymbol gap.
    let program = r#"
(def f (lambda () (g)))
"#;

    assert_eq!(native_error_kind(program, "(f)"), ErrorKind::UnknownSymbol);
    assert_eq!(meta_eval_program(program, "(f)"), "(error unbound-symbol g)");
}

#[test]
fn lexical_parameter_shadowing_still_beats_the_shared_definition_frame() {
    let program = r#"
(def g 1)
(def f (lambda (g) g))
(def g 2)
"#;

    assert_eq!(native_value(program, "(f 9)"), "9");
    assert_eq!(meta_eval_program(program, "(f 9)"), "9");
}

#[test]
fn nested_closure_keeps_its_child_binding_while_parent_frame_keeps_growing() {
    let program = r#"
(def make (lambda (g) (lambda () g)))
(def held (make 7))
(def g 42)
"#;

    assert_eq!(native_value(program, "(held)"), "7");
    assert_eq!(meta_eval_program(program, "(held)"), "7");
}
