//! Exact D8 WRITE-TO-STRING admission; no historical-byte reconstitution.
use sens::{parse_mixed_exact_domain_legacy_core_source, eval_parsed_expressions, Session};

#[test]
fn d8_writer_serializes_structural_data_by_ratified_identity() {
    let mut session = Session::default();
    for (source, expected) in [
        ("(11110100 (001 (a b)))", "\"(a b)\""),
        ("(11110100 (001 ()))", "\"()\""),
        ("(11110100 7)", "\"#q2:111/1\""),
    ] {
        let parsed = parse_mixed_exact_domain_legacy_core_source("lib/core.lisp", source)
            .unwrap_or_else(|e| panic!("D8 surface must be an exact call head: {source}: {e:?}"));
        let result = eval_parsed_expressions(&parsed, &mut session)
            .unwrap_or_else(|e| panic!("D8 writer must use a direct mechanism: {source}: {e:?}"));
        assert_eq!(result.value.to_string(), expected, "writer mismatch {source}");
    }
}

#[test]
fn pinned_d8_writer_survives_d4_and_legacy_wrapped_closures() {
    let mut session = Session::default();
    for definition in [
        "(0011 внутрішній-d8 (0010 (value) (11110100 value)))",
        "(00001001 внутрішній-d8-старий (00001000 (value) (11110100 value)))",
    ] {
        let parsed = parse_mixed_exact_domain_legacy_core_source("lib/core.lisp", definition)
            .expect("D8 writer closure parses");
        eval_parsed_expressions(&parsed, &mut session)
            .unwrap_or_else(|e| panic!("D8 writer closure installs {definition}: {e:?}"));
    }
    for head in ["внутрішній-d8", "внутрішній-d8-старий"] {
        let call = format!("({head} (quote (a b)))");
        let result = sens::eval_program(&call, &mut session)
            .unwrap_or_else(|e| panic!("D8 closure body lost identity {call}: {e:?}"));
        assert_eq!(result.value.to_string(), "\"(a b)\"");
    }
}

#[test]
fn d8_read_write_round_trip_uses_only_exact_ratified_d8_coordinates() {
    let mut session = Session::default();
    for (source, expected) in [
        ("(11101010 \"(a b)\")", "(a b)"),
        ("(11101010 (11110100 (001 (a b))))", "(a b)"),
        ("(11101010 (11110100 7))", "7"),
    ] {
        let parsed = parse_mixed_exact_domain_legacy_core_source("lib/core.lisp", source)
            .expect("path-bound D8 READ/WRITE must parse");
        let result = eval_parsed_expressions(&parsed, &mut session)
            .unwrap_or_else(|e| panic!("D8 READ/WRITE {source}: {e:?}"));
        assert_eq!(result.value.to_string(), expected, "D8 wire round trip");
    }
}

#[test]
fn d8_string_to_symbol_is_exact_and_rejects_non_strings() {
    let mut session = Session::default();
    let parsed = parse_mixed_exact_domain_legacy_core_source("lib/core.lisp", "(11101101 \"zoo\")")
        .expect("D8 scalar conversion must parse by exact binary head");
    let result = eval_parsed_expressions(&parsed, &mut session)
        .expect("ratified D8 symbol conversion");
    assert_eq!(result.value.to_string(), "zoo");

    let bad = parse_mixed_exact_domain_legacy_core_source("lib/core.lisp", "(11101101 7)")
        .expect("invalid argument is a runtime type failure, not parser rewriting");
    assert_eq!(
        eval_parsed_expressions(&bad, &mut session)
            .expect_err("a number is not a text carrier").kind,
        sens::ErrorKind::Type
    );
}

#[test]
fn d8_read_rejects_excess_arguments() {
    let mut session = Session::default();
    let parsed = parse_mixed_exact_domain_legacy_core_source("lib/core.lisp", "(11101010 \"a\" \"b\")")
        .expect("exact D8 READ expression parses");
    let error = eval_parsed_expressions(&parsed, &mut session)
        .expect_err("D8 READ cannot accept two inputs");
    assert_eq!(error.kind, sens::ErrorKind::Arity);
}

#[test]
fn d8_writer_rejects_incorrect_arity() {
    let mut session = Session::default();
    let parsed = parse_mixed_exact_domain_legacy_core_source("lib/core.lisp", "(11110100 1 2)")
        .expect("D8 writer reference parses");
    let error = eval_parsed_expressions(&parsed, &mut session)
        .expect_err("writer accepts exactly one value");
    assert_eq!(error.kind, sens::ErrorKind::Arity);
}
