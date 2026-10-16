//! D5 compatibility-reader boundary: short words are not silently treated as domain IDs.

use sens::{parse, ExprKind};

#[test]
fn compatibility_parser_does_not_mint_short_domain_words() {
    let parsed = parse("10000").expect("ordinary decimal source");
    assert!(matches!(
        parsed[0].kind,
        ExprKind::Number(value, _) if value == 10_000.0
    ));
}
