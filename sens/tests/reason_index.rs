use sens::{
    eval_parsed_expressions, eval_program, parse_canonical_binary, parse_exact_domain_source,
    parse_mixed_exact_domain_reasoning_source, ExprKind, Session,
};

fn session() -> Session {
    let mut session = Session::default();
    sens::eval_pinned_legacy_core_source(
        "lib/core.lisp", include_str!("../../../lib/core.lisp"), &mut session,
    ).expect("канонічний Core має завантажуватись власним точним reader");
    for (path, source) in [
        ("lib/unify.lisp", include_str!("../../../lib/unify.lisp")),
        ("lib/reason.lisp", include_str!("../../../lib/reason.lisp")),
    ] {
        let parsed = parse_mixed_exact_domain_reasoning_source(path, source)
            .unwrap_or_else(|error| panic!("{path}: exact source reader: {error}"));
        eval_parsed_expressions(&parsed, &mut session)
            .unwrap_or_else(|error| panic!("{path}: exact source evaluator: {error}"));
    }
    session
}

fn eval(session: &mut Session, source: &str) -> String {
    eval_program(source, session)
        .unwrap_or_else(|e| panic!("evaluation failed: {e}\nsource: {source}"))
        .value
        .to_string()
}

#[test]
fn indexed_bucket_preserves_source_rule_order_exactly() {
    let mut s = session();
    let source = r#"
      (def rules
        (quote
          (((p first))
           ((q irrelevant))
           ((p second))
           ((r irrelevant))
           ((p third)))))
      (reason-index-candidates (quote (p target)) (reason-make-index rules))
    "#;
    assert_eq!(
        eval(&mut s, source),
        "(((p first)) ((p second)) ((p third)))"
    );
}

#[test]
fn variable_goal_falls_back_to_every_rule() {
    let mut s = session();
    // Довжина як некваліфіковане число нині не є D24+ арифметикою.
    // Перевіряємо сильніший інваріант: усі правила + первісний порядок.
    let source = r#"
      (def rules (quote (((p one)) ((q two)) ((r three)))))
      (reason-index-candidates
        (quote (var x))
        (reason-make-index rules))
    "#;
    assert_eq!(
        eval(&mut s, source),
        "(((p one)) ((q two)) ((r three)))"
    );
}

#[test]
fn variable_rule_head_disables_index_instead_of_changing_unification() {
    let mut s = session();
    let source = r#"
      (def rules
        (quote
          (((p one))
           ((var whole-head))
           ((q two)))))
      (reason-index-mode (reason-make-index rules))
    "#;
    assert_eq!(eval(&mut s, source), "linear");
}

#[test]
fn too_many_distinct_predicates_fall_back_to_linear_mode() {
    let mut s = session();
    let mut source = String::from("(reason-index-mode (reason-make-index (quote (");
    for i in 0..65 {
        source.push_str(&format!("((pred{i} value))"));
    }
    source.push_str("))))");
    assert_eq!(eval(&mut s, &source), "linear");
}

#[test]
fn binary_sens_migration_witness_checks_atom_eq_and_domain_boundaries() {
    let mut s = session();

    // The law itself is binary SENS: only admitted D3 ATOM/EQ primitives
    // decide the semantic result. No unratified user function is made callable.
    let law = include_str!("../../../tests/semantic/narrate-d3-atom-eq-law.bits");
    assert!(
        law.bytes().all(|b| b.is_ascii_whitespace() || matches!(b, b'0' | b'1')),
        "the semantic witness must be visible binary words plus separators"
    );
    let forms = parse_canonical_binary(law).expect("canonical binary ATOM/EQ law");
    let verdict = eval_parsed_expressions(&forms, &mut s)
        .expect("SENS rejected the canonical ATOM/EQ law");
    assert_eq!(
        verdict.value.as_predicate_bit(),
        Some(true),
        "SENS primitive law must produce exact D1:YES"
    );

    // These are mechanical invariants over production source, not a host
    // truth table: prove EQ cannot see a pair and each helper keeps its width.
    let reason_source = include_str!("../../../lib/reason.lisp");
    let rs = reason_source.find("(0011 reason-index?").expect("reason-index? exists");
    let re = reason_source[rs..].find("(0011 reason-index-linear").expect("reason-index? ends").saturating_add(rs);
    let body = &reason_source[rs..re];
    let empty_guard = body.find("((0101 value)").expect("D4 NULL guard");
    let car_atom_guard = body.find("((010 (100 value))").expect("D3 ATOM(CAR) guard");
    let eq_call = body.find("((101 (100 value) *reason-index-schema*)").expect("D3 EQ call");
    assert!(empty_guard < car_atom_guard && car_atom_guard < eq_call,
        "D4 NULL and D3 ATOM(CAR) must guard D3 EQ, in that order");

    let narration_source = include_str!("../../../lib/narrate.lisp");
    let hs = narration_source.find("(0011 narrate-proper-list?").expect("D3/D4 helpers exist");
    let he = narration_source[hs..].find("(0011 narrate-fact").expect("helper span ends").saturating_add(hs);
    let helper_source = &narration_source[hs..he];
    let helper_forms = parse_exact_domain_source(helper_source)
        .expect("arity helpers must retain exact-domain identities");
    assert_eq!(helper_forms.len(), 3, "one proper-list helper and two arity helpers");
    let mut widths = Vec::new();
    fn collect_widths(expr: &sens::Expr, widths: &mut Vec<usize>) {
        if let ExprKind::DomainIdentity(identity) = &expr.kind {
            widths.push(identity.width());
        }
        if let ExprKind::List(items) = &expr.kind {
            for item in items.iter() {
                collect_widths(item, widths);
            }
        }
    }
    for form in &helper_forms {
        collect_widths(form, &mut widths);
    }
    widths.sort_unstable();
    widths.dedup();
    assert_eq!(widths, vec![1, 3, 4], "D1 results; D3 structure; D4 empty-list predicate");
    assert!(!helper_source.contains("000000"), "D6 LENGTH must not return to arity helpers");
    assert!(!helper_source.contains("11110111"), "D8 EQUAL must not return to arity helpers");
}

#[test]
fn reason_index_migration_exhausts_all_2046_exact_width_coordinate_candidates() {
    // This counts 2046 mechanical coordinates (NOT 2046 callable functions).
    // The test detects truncation, cross-domain aliasing and accidental
    // logical padding, independently of display labels or legacy SID8.
    use sens::domain_ladder::DomainCoordinate;
    use std::collections::HashSet;

    let mut seen = HashSet::new();
    let mut bit_count = 0usize;
    for width in 1u8..=10 {
        let capacity = 1u16 << width;
        for payload in 0..capacity {
            let coordinate = DomainCoordinate::new(width, payload)
                .expect("every in-width pair must be representable");
            assert!(seen.insert(coordinate),
                "collision in D{width}: {payload:0w$b}", w = usize::from(width));
            assert_eq!(coordinate.width(), width);
            assert_eq!(coordinate.bits(), payload);
            assert_ne!(DomainCoordinate::new(width, capacity), Some(coordinate));
            bit_count += usize::from(width);
        }
        assert!(DomainCoordinate::new(width, capacity).is_none(),
            "oversize value must not truncate into D{width}");
        assert!(DomainCoordinate::new(width, u16::MAX).is_none());
    }
    assert_eq!(seen.len(), 2046);
    assert_eq!(bit_count, 18_434);
    assert_eq!((bit_count + 7) / 8, 2305);
    assert_eq!(8 * ((bit_count + 7) / 8) - bit_count, 6);
    for width in 1u8..=9 {
        assert_ne!(DomainCoordinate::new(width, 1),
                   DomainCoordinate::new(width + 1, 1));
    }
    assert_eq!(DomainCoordinate::new(10, 1024), None);
}

#[test]
fn reason_index_builder_diagnostic_first_failing_stage() {
    let mut s = session();
    let probes = [
        ("(reason-term-predicate (quote (p a)))", "p"),
        ("(reason-rule-predicate (quote ((p a))))", "p"),
        ("(reason-index-add-reversed (quote p) (quote ((p a))) (quote ()))", "((p ((p a))))"),
        ("(reason-index-normalize-buckets (reason-index-add-reversed (quote p) (quote ((p a))) (quote ())))", "((p ((p a))))"),
        ("(reason-index-build-scan (quote ()) (quote ()) (quote ()) *reason-index-fuel*)",
         "(reason-index/1 indexed () ())"),
        ("(reason-make-index (quote (((p a)))))",
         "(reason-index/1 indexed (((p a))) ((p ((p a)))))"),
    ];
    for (probe, expected) in probes {
        assert_eq!(eval(&mut s, probe), expected, "first failing index stage: {probe}");
    }
}
