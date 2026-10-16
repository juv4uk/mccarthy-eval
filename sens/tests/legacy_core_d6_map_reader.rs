//! Regression for the legacy Core macro MAP head (D6 101000).
//! D6:101000 is a ratified coordinate without an admitted evaluator mechanism.
//! The legacy let macro must use its existing Lisp-defined map closure while
//! both ordinary and typed readers continue to refuse invented D6 dispatch.
use sens::{
    eval_parsed_expressions, eval_program, parse_mixed_exact_domain_legacy_core_source,
    Session,
};
const SOURCE: &str = include_str!("../../../lib/core.lisp");

#[test]
fn legacy_core_reader_is_explicitly_path_bound() {
    assert!(parse_mixed_exact_domain_legacy_core_source("lib/core.lisp", SOURCE).is_ok());
    for path in ["lib/core4.lisp", "lib/forward.lisp", "../core.lisp"] {
        assert!(
            parse_mixed_exact_domain_legacy_core_source(path, SOURCE).is_err(),
            "unapproved core source: {path}"
        );
    }
}

#[test]
fn legacy_core_let_uses_lisp_owned_map_without_unadmitted_d6_dispatch() {
    // Only the explicit source-pinned reader interprets W3-W6 call heads.
    // The ordinary reader is not allowed to infer domain width from numerals.
    for typed in [true] {
        let mut session = Session::default();
        if typed {
            let parsed = parse_mixed_exact_domain_legacy_core_source("lib/core.lisp", SOURCE)
                .expect("legacy core source must parse");
            eval_parsed_expressions(&parsed, &mut session)
                .expect("typed legacy bootstrap must load");
        } else {
            eval_program(SOURCE, &mut session).expect("ordinary legacy bootstrap must load");
        }
        for (program, expected) in [
            ("(let ((x 7)) x)", "7"),
            ("(let ((x 7) (y 9)) (list x y))", "(7 9)"),
        ] {
            let result = eval_program(program, &mut session)
                .unwrap_or_else(|e| panic!("Lisp-owned MAP must work in {program}, typed={typed}: {e:?}"));
            assert_eq!(result.value.to_string(), expected);
        }
    }
}

#[test]
fn legacy_core_let_star_uses_derived_lambda_not_unadmitted_d6_let() {
    let mut session = Session::default();
    sens::eval_pinned_legacy_core_source("lib/core.lisp", SOURCE, &mut session)
        .expect("source-pinned core");
    for (form, expected) in [
        ("(let* () 42)", "42"),
        ("(let* ((x 7)) x)", "7"),
        ("(let* ((x 7) (y x)) y)", "7"),
    ] {
        let actual = eval_program(form, &mut session)
            .unwrap_or_else(|e| panic!("lambda-derived let* {form}: {e:?}")).value.to_string();
        assert_eq!(actual, expected, "sequential binding {form}");
    }
}

#[test]
fn likhtar_macro_expands_without_unadmitted_d6_let_dispatch() {
    let mut session = Session::default();
    sens::eval_pinned_legacy_core_source("lib/core.lisp", SOURCE, &mut session)
        .expect("pinned Lisp Core bootstrap must load");
    for (form, expected) in [
        ("(нехай-похідне () 42)", "42"),
        ("(нехай-похідне ((x 7)) x)", "7"),
        ("(нехай-похідне ((x 7) (y 9)) (00100111 x y))", "(7 9)"),
    ] {
        let value = eval_program(form, &mut session)
            .unwrap_or_else(|e| panic!("ЛІХТАР macro failed {form}: {e:?}"))
            .value.to_string();
        assert_eq!(value, expected, "ЛІХТАР did not preserve parallel bindings: {form}");
    }
}

#[test]
fn private_derived_length_counts_empty_and_nonempty_lists() {
    let mut session = Session::default();
    sens::eval_pinned_legacy_core_source("lib/core.lisp", SOURCE, &mut session)
        .expect("typed core bootstrap");
    for (form, expected) in [
        ("(внутрішня-довжина (00000001 ()))", "0"),
        ("(внутрішня-довжина (00000001 (a)))", "1"),
        ("(внутрішня-довжина (00000001 (a b)))", "2"),
    ] {
        let outcome = eval_program(form, &mut session)
            .unwrap_or_else(|e| panic!("derived LENGTH {form}: {e:?}"));
        assert_eq!(outcome.value.to_string(), expected);
    }
}

#[test]
fn derived_map_preserves_macro_binding_parameter_and_value_lists() {
    let mut session = Session::default();
    sens::eval_pinned_legacy_core_source("lib/core.lisp", SOURCE, &mut session)
        .expect("typed core bootstrap");
    for (source, expected) in [
        ("(внутрішнє-відображення (00001000 (binding) (00000101 binding)) (00000001 ((x 7))))", "(x)"),
        ("(внутрішнє-відображення (00001000 (binding) (00101111 binding)) (00000001 ((x 7))))", "(7)"),
        ("(map (00001000 (binding) (00000101 binding)) (00000001 ((x 7))))", "(x)"),
    ] {
        let output = eval_program(source, &mut session)
            .unwrap_or_else(|e| panic!("derived MAP {source}: {e:?}")).value.to_string();
        assert_eq!(output, expected, "derived MAP mismatch for {source}");
    }
}

#[test]
fn unadmitted_d6_map_does_not_silently_fall_back_to_historical_byte() {
    let mut session = Session::default();
    let parsed = parse_mixed_exact_domain_legacy_core_source("lib/core.lisp", SOURCE)
        .expect("core reader must parse");
    eval_parsed_expressions(&parsed, &mut session).expect("core bootstrap");
    let attempt = sens::parse_mixed_exact_domain("(101000 (0010 (x) x) (001 (a)))")
        .expect("current exact-domain source parses");
    let failure = eval_parsed_expressions(&attempt, &mut session)
        .expect_err("D6 MAP cannot acquire unproven evaluator meaning by accident");
    assert!(failure.message.contains("domain identity is not callable"), "{failure:?}");
}
