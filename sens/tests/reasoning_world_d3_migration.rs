//! Mechanical source-boundary witness for the Lisp-owned immutable-world layer.
//! Behavioral expectations are SENS-owned in tests/semantic/world-exact-arity-witness.lisp.

use sens::{parse_mixed_exact_domain_reasoning_source, ErrorKind, Expr, ExprKind};

const OLD: &str = include_str!("../../../tests/fixtures/world-d3-before.lisp");
const D3: &str = include_str!("../../../lib/world.lisp");

#[test]
fn world_d3_candidate_does_not_rewrite_w8_quote_data() {
    assert!(OLD.contains("(00000111"));
    assert!(!D3.contains("(00000111"));
    assert!(D3.contains("(110"));
    assert!(D3.contains("(101"));
    assert!(D3.contains("(010"));

    assert_eq!(
        parse_mixed_exact_domain_reasoning_source("lib/not-world.lisp", D3)
            .expect_err("unapproved path must fail closed").kind,
        ErrorKind::Parse
    );
}


fn collect_call_head_coordinates(expr: &Expr, coordinates: &mut Vec<(usize, u16)>) {
    let ExprKind::List(items) = &expr.kind else {
        return;
    };
    if let Some(Expr { kind: ExprKind::DomainIdentity(identity), .. }) = items.first() {
        coordinates.push((identity.width(), identity.packed_bits()));
    }
    for item in items.iter() {
        collect_call_head_coordinates(item, coordinates);
    }
}

#[test]
fn world_shape_source_uses_only_exact_d3_d4_call_heads() {
    let parsed = parse_mixed_exact_domain_reasoning_source("lib/world.lisp", D3)
        .expect("world source must retain exact domain identity");
    let definition = parsed.iter().find(|expression| {
        let ExprKind::List(items) = &expression.kind else { return false; };
        matches!(
            items.get(1).map(|item| &item.kind),
            Some(ExprKind::Symbol(name)) if name.as_ref() == "world?"
        )
    }).expect("world? definition is present");

    let mut coordinates = Vec::new();
    collect_call_head_coordinates(definition, &mut coordinates);
    coordinates.sort_unstable();
    coordinates.dedup();

    let widths = coordinates.iter().map(|(width, _)| *width)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        widths,
        [3usize, 4usize].into_iter().collect(),
        "world? may use D3 structure and D4 NULL/definition only"
    );
    for required in [(3, 0b001), (3, 0b010), (3, 0b011), (3, 0b101), (3, 0b110),
                     (4, 0b0010), (4, 0b0011), (4, 0b0101)] {
        assert!(
            coordinates.contains(&required),
            "world? source is missing exact D{}:{:0width$b}",
            required.0,
            required.1,
            width = required.0
        );
    }
}
