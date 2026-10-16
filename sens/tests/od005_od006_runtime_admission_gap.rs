use sens::{parse, ExprKind};

fn parsed_head(source: &str) -> Vec<sens::Expr> {
    let expressions = parse(source).expect("source must parse");
    assert_eq!(expressions.len(), 1);
    expressions
}

#[test]
fn od005_od006_owner_coordinates_are_not_yet_direct_sid_tokens() {
    for source in ["(00111)", "(001111)"] {
        let expressions = parsed_head(source);
        let ExprKind::List(items) = &expressions[0].kind else {
            panic!("{source}: expected list")
        };

        assert!(
            !matches!(&items[0].kind, ExprKind::Sid(_)),
            "{source}: D5/D6 bare owner coordinate must not be reported as admitted SID before exact-width carrier migration"
        );
        assert!(
            matches!(&items[0].kind, ExprKind::Number(_, _) | ExprKind::Rational(_)),
            "{source}: current reader collision witness should remain numeric until the explicit migration changes it"
        );
    }
}

#[test]
fn exact_eight_bit_legacy_control_is_still_sid() {
    let expressions = parsed_head("(00000111)");
    let ExprKind::List(items) = &expressions[0].kind else {
        panic!("expected list")
    };
    assert!(
        matches!(&items[0].kind, ExprKind::Sid(_)),
        "8-bit legacy control must remain a SID in this transitional audit"
    );
}
