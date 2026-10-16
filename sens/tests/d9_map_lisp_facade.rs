//! D9 map coordinates without admitted mechanisms must remain fail-closed.
//! These tests exercise separately named Lisp-owned AVL closures only.
use sens::{
    eval_program, eval_pinned_legacy_core_source, eval_pinned_reasoning_source,
    Expr, ExprKind, Session,
};

fn session() -> Session {
    let mut session = Session::default();
    eval_pinned_legacy_core_source(
        "lib/core.lisp", include_str!("../../../lib/core.lisp"), &mut session
    ).expect("derived core bootstrap");
    eval_pinned_reasoning_source(
        "lib/persistent-map.lisp", include_str!("../../../lib/persistent-map.lisp"), &mut session
    ).expect("Lisp-owned persistent map bootstrap");
    session
}

#[test]
fn source_cutover_blocks_old_d6_let_and_unadmitted_d9_map_heads() {
    for (path, source, expected_macro_count) in [
        ("lib/persistent-map.lisp", include_str!("../../../lib/persistent-map.lisp"), 0),
        ("lib/content-store.lisp", include_str!("../../../lib/content-store.lisp"), 0),
        ("lib/lisp-fs.lisp", include_str!("../../../lib/lisp-fs.lisp"), 13),
    ] {
        for head in [
            "001000",    // D6 LET: replaced with the Lisp-owned ЛІХТАР macro
            "101001100", // D9 MAP->LIST
            "101100101", // D9 MAP-GET
            "101100010", // D9 MAP-CONTAINS?
            "101101110", // D9 MAP-INSERT
            "110111011", // D9 ANSWER-NOT: D1-only cases use D4 NOT
        ] {
            assert!(
                !source.contains(&format!("({head} ")),
                "obsolete executable head {head} reappeared in {path}"
            );
        }
        assert_eq!(
            source.matches("(нехай-похідне").count(),
            expected_macro_count,
            "source macro cutover drift in {path}"
        );
    }
}

fn collect_exact_head_identities(expr: &Expr, out: &mut Vec<(usize, u16)>) {
    let ExprKind::List(items) = &expr.kind else {
        return;
    };
    if let Some(head) = items.first() {
        if let ExprKind::DomainIdentity(identity) = &head.kind {
            out.push((identity.width(), identity.packed_bits()));
        }
    }
    for item in items.iter() {
        collect_exact_head_identities(item, out);
    }
}

#[test]
fn persistent_map_core_uses_only_exact_d3_d4_heads_and_no_numeric_ordering() {
    let source = include_str!("../../../lib/persistent-map.lisp");

    for forbidden in [
        "01010",      // D5 PLUS / historical arithmetic path
        "01011",      // D5 DIFFERENCE
        "11010",      // D5 LESSP
        "110011100",  // D9 STRING-EMPTY?
        "110101011",  // D9 STRING-FIRST
        "110011101",  // D9 STRING-REST
        "11100000",   // D8 CHAR-CODE
    ] {
        assert!(
            !source.contains(&format!("({forbidden} ")),
            "persistent map reintroduced arithmetic/text-order head {forbidden}"
        );
    }

    let parsed = sens::parse_mixed_exact_domain_reasoning_source(
        "lib/persistent-map.lisp",
        source,
    )
    .expect("persistent map must parse as exact-domain source");

    let mut heads = Vec::new();
    for expr in &parsed {
        collect_exact_head_identities(expr, &mut heads);
    }

    assert!(!heads.is_empty(), "proof must inspect executable exact-domain heads");
    assert!(
        heads.iter().all(|(width, _)| matches!(width, 3 | 4)),
        "persistent map execution must be D3/D4-only; got {heads:?}"
    );
}

#[test]
fn internal_map_facade_is_not_d9_auto_dispatch() {
    let mut session = session();
    let empty = eval_program("(внутрішня-мапа-список map-empty)", &mut session)
        .expect("private derived map->list closure");
    assert_eq!(empty.value.to_string(), "()");
    let failure = sens::parse_mixed_exact_domain("(101001100 (001 ()))")
        .expect("exact D9 source must not silently reinterpret a numeric value");
    // D9 has no admitted callable mechanism. A number is never promoted
    // to a function merely because the spelling happens to be binary.
    let error = sens::eval_parsed_expressions(&failure, &mut session)
        .expect_err("unadmitted 9-bit head must fail closed");
    assert!(
        error.message.contains("not callable"),
        "expected the named non-callable diagnostic: {error:?}"
    );
}

#[test]
fn persistent_map_bounded_update_space_matches_independent_model() {
    let mut session = session();
    let keys = ["a", "b", "c"];

    // 3^4 = 81 deterministic update traces. This is deliberately a small,
    // exhaustive state-space witness rather than another single fixture.
    for trace in 0usize..81 {
        let mut n = trace;
        let mut source = "map-empty".to_string();
        let mut model = std::collections::BTreeMap::<&str, String>::new();

        for step in 0..4 {
            let key = keys[n % keys.len()];
            n /= keys.len();
            let value = format!("v{step}");
            source = format!(
                "(внутрішня-мапа-вставити \"{key}\" \"{value}\" {source})"
            );
            model.insert(key, value);
        }

        for key in keys {
            let get = eval_program(
                &format!("(внутрішня-мапа-отримати \"{key}\" {source})"),
                &mut session,
            )
            .unwrap_or_else(|e| panic!("trace={trace} key={key} get: {e:?}"))
            .value
            .to_string();

            let expected = model
                .get(key)
                .map(|value| format!("(\"{value}\")"))
                .unwrap_or_else(|| "()".to_string());
            assert_eq!(get, expected, "trace={trace} key={key}");

            let contains = eval_program(
                &format!("(внутрішня-мапа-містить \"{key}\" {source})"),
                &mut session,
            )
            .unwrap_or_else(|e| panic!("trace={trace} key={key} contains: {e:?}"))
            .value;
            assert_eq!(
                contains.as_predicate_bit(),
                Some(model.contains_key(key)),
                "membership must be exact D1 for trace={trace} key={key}"
            );
        }
    }
}

#[test]
fn persistent_map_replaces_equal_key_without_growing_a_duplicate_entry() {
    let mut session = session();
    let source =
        "(внутрішня-мапа-список
           (внутрішня-мапа-вставити \"a\" \"new\"
             (внутрішня-мапа-вставити \"b\" \"bee\"
               (внутрішня-мапа-вставити \"a\" \"old\" map-empty))))";
    let listed = eval_program(source, &mut session)
        .expect("persistent association map replacement")
        .value
        .to_string();
    assert_eq!(listed, "((\"a\" . \"new\") (\"b\" . \"bee\"))");
}

#[test]
fn internal_map_facade_inserts_and_reads_one_element() {
    let mut session = session();
    let inserted = eval_program(
        "(внутрішня-мапа-отримати \"a\" (внутрішня-мапа-вставити \"a\" 7 map-empty))",
        &mut session
    ).expect("one-key persistent map insert and read use Lisp closures");
    assert_eq!(inserted.value.to_string(), "(7)");
    let listed = eval_program(
        "(внутрішня-мапа-список (внутрішня-мапа-вставити \"a\" 7 map-empty))",
        &mut session
    ).expect("one-key persistent map should traverse without D9 dispatch");
    assert_eq!(listed.value.to_string(), "((\"a\" . 7))");
}
