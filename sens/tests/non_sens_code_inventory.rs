//! The historical registry-name/runtime inventory is retired: language names
//! are projections, not semantic authority. Rust checks exact domain widths only.
use sens::{parse_canonical_binary, syntax::ExprKind};

#[test]
fn binary_identity_ladder_preserves_all_supported_widths() {
    for width in 1usize..=9 {
        let payload = (1usize << width) - 1;
        let source = format!("{payload:0width$b}");
        let forms = parse_canonical_binary(&source).expect("exact-width word parses");
        assert_eq!(forms.len(), 1);
        let ExprKind::DomainIdentity(identity) = &forms[0].kind else {
            panic!("binary word must remain a domain identity");
        };
        assert_eq!(identity.width(), width);
        assert_eq!(identity.packed_bits() as usize, payload);
    }
}

#[test]
fn identical_payloads_remain_distinct_when_word_width_differs() {
    let mut coordinates = Vec::new();
    for width in 1usize..=9 {
        let source = format!("{value:0width$b}", value = 1usize);
        let forms = parse_canonical_binary(&source).expect("width-qualified word parses");
        let ExprKind::DomainIdentity(identity) = &forms[0].kind else {
            panic!("binary word must remain a domain identity");
        };
        coordinates.push((identity.width(), identity.packed_bits()));
    }
    assert!(coordinates.iter().all(|(_, bits)| *bits == 1));
    for left in 0..coordinates.len() {
        for right in (left + 1)..coordinates.len() {
            assert_ne!(coordinates[left].0, coordinates[right].0);
        }
    }
}