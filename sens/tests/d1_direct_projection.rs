//! D1 is a direct one-bit value, not a function and not D3:EMPTY.
//! #d1:0/#d1:1 is only the temporary human-Lisp projection; the
//! canonical SENS reader uses the bare exact W1 words 0 and 1.
use sens::{
    eval_program, eval_parsed_expressions, parse, parse_mixed_exact_domain, parse_canonical_binary,
    DomainIdentity, Exactness, ExprKind, Session, Value,
};

#[test]
fn historical_reader_projects_only_two_explicit_d1_words() {
    for (source, exact, expected) in [("#d1:0", "0", false), ("#d1:1", "1", true)] {
        let parsed = parse(source).expect("typed D1 projection");
        assert!(matches!(parsed[0].kind, ExprKind::DomainIdentity(DomainIdentity::D1(_))));
        let direct = eval_program(source, &mut Session::default())
            .expect("direct D1, without evaluating ATOM/NOT").value;
        let canonical_ast = parse_canonical_binary(exact).expect("canonical one-bit source");
        let canonical = eval_parsed_expressions(&canonical_ast, &mut Session::default())
            .expect("canonical D1 evaluation").value;
        assert_eq!(direct.as_predicate_bit(), Some(expected));
        assert_eq!(canonical, direct);
    }
}

#[test]
fn bare_numerals_are_not_reinterpreted_in_historical_source() {
    for source in ["0", "1"] {
        let ast = parse(source).expect("historical number reader");
        assert!(matches!(ast[0].kind, ExprKind::Number(_, Exactness::Exact)));
        let value = eval_program(source, &mut Session::default()).unwrap().value;
        assert_eq!(value.as_predicate_bit(), None);
    }
}

#[test]
fn structural_empty_is_never_boolean_false() {
    let empty = eval_program("(quote ())", &mut Session::default())
        .expect("D3 quote structural empty").value;
    let zero = eval_program("#d1:0", &mut Session::default())
        .expect("direct D1 NO").value;
    assert!(matches!(empty, Value::Nil));
    assert_eq!(empty.as_predicate_bit(), None);
    assert_eq!(zero.as_predicate_bit(), Some(false));
    assert_ne!(empty, zero);
}

#[test]
fn malformed_d1_projections_fail_closed() {
    for source in ["#d1:", "#d1:00", "#d1:01", "#d1:10", "#d1:2",
                   "#d1:-1", "#d1:1x", "#d1:000", "#d1:YES"] {
        assert!(parse(source).is_err(), "must reject {source}");
    }
    // The canonical wire grammar accepts only raw exact-width bits, never
    // this human-readable compatibility sigil.
    assert!(parse_canonical_binary("#d1:1").is_err());
}

#[test]
fn cond_consumes_direct_d1_without_hidden_truthiness() {
    for (condition, expected) in [("#d1:1", "yes"), ("#d1:0", "no")] {
        let source = format!("(110 ({condition} (001 yes)) (#d1:1 (001 no)))");
        let parsed = parse_mixed_exact_domain(&source).expect("exact D3 COND with D1 branch tests");
        let actual = eval_parsed_expressions(&parsed, &mut Session::default())
            .expect("exact D1 predicate must control COND").value;
        assert_eq!(actual.to_string(), expected);
    }
}

#[test]
fn canonical_bare_d1_cond_succeeds_while_mixed_legacy_bare_number_rejects() {
    // The same D3:110 COND control is an exact W1 bit inside canonical D2
    // words. This is the already admitted source, not a newly inferred width.
    // The physical Zig corridor is outside this SENS reader/evaluator test.
    let canonical = "10 110 00 10 0 00 0 01 00 10 1 00 1 01 01";
    let ast = parse_canonical_binary(canonical).expect("bare W1 in canonical D2 COND");
    let canonical_value = eval_parsed_expressions(&ast, &mut Session::default())
        .expect("canonical D1:0 skips, D1:1 selects").value;
    assert_eq!(canonical_value.as_predicate_bit(), Some(true));

    // TEMPORARY #5605: a legacy mixed .lisp consumer still requires a
    // declared D1 boundary. A textual tag must never be copied into the
    // canonical binary grammar.
    let tagged = "(110 (#d1:0 #d1:0) (#d1:1 #d1:1))";
    let tagged_ast = parse_mixed_exact_domain(tagged).expect("legacy carrier syntax");
    let tagged_value = eval_parsed_expressions(&tagged_ast, &mut Session::default())
        .expect("temporary bridge preserves exact D1").value;
    assert_eq!(tagged_value.as_predicate_bit(), Some(true));

    // One purportedly mechanical removal of the two tags changes the
    // literal's DOMAIN into historical Number, not W1. The reader must not
    // silently coerce a host number into SENS PredicateBit.
    let bare_mixed = "(110 (0 0) (1 1))";
    let bare_ast = parse_mixed_exact_domain(bare_mixed).expect("legacy mixed syntax");
    let error = eval_parsed_expressions(&bare_ast, &mut Session::default())
        .expect_err("a legacy Number 0 is not canonical D1:NO");
    assert_eq!(error.kind, sens::ErrorKind::Type);
    assert!(
        error.message.contains("COND expects exact D1:1 / D1:0"),
        "unexpected non-D1 diagnosis: {}", error.message
    );

    let redundant = canonical.replacen("10 1 00 1", "10 #d1:1 00 1", 1);
    assert!(parse_canonical_binary(&redundant).is_err(),
        "compatibility tag is never canonical grammar");
}
