//! Розрізнення доменної драбини без підміни структурного D2 даними.
//! Rust перевіряє ширину й тотожність; синтаксис D2 належить канонічному читачеві.
use sens::{parse_canonical_binary, syntax::ExprKind};

const PAYLOAD_WIDTHS: [usize; 8] = [1, 3, 4, 5, 6, 7, 8, 9];

#[test]
fn each_payload_domain_width_roundtrips_as_a_distinct_identity() {
    for width in PAYLOAD_WIDTHS {
        let payload = (1usize << width) - 1;
        let source = format!("{payload:0width$b}");
        let forms = parse_canonical_binary(&source).expect("canonical domain word parses");
        assert_eq!(forms.len(), 1);
        let ExprKind::DomainIdentity(identity) = &forms[0].kind else {
            panic!("canonical binary word must remain a domain identity");
        };
        assert_eq!(identity.width(), width);
        assert_eq!(usize::from(identity.packed_bits()), payload);
    }
}

#[test]
fn equal_payloads_do_not_collapse_across_payload_domains() {
    let coordinates = PAYLOAD_WIDTHS
        .into_iter()
        .map(|width| {
            let source = format!("{value:0width$b}", value = 1usize);
            let parsed = parse_canonical_binary(&source).expect("domain identity parses");
            match &parsed[0].kind {
                ExprKind::DomainIdentity(identity) => (identity.width(), identity.packed_bits()),
                _ => panic!("expected domain identity"),
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(
        coordinates.iter().map(|(_, bits)| *bits).collect::<Vec<_>>(),
        vec![1; PAYLOAD_WIDTHS.len()]
    );
    assert_eq!(
        coordinates.iter().map(|(width, _)| *width).collect::<Vec<_>>(),
        PAYLOAD_WIDTHS.to_vec()
    );
    for left in 0..coordinates.len() {
        for right in (left + 1)..coordinates.len() {
            assert_ne!(coordinates[left], coordinates[right]);
        }
    }
}

#[test]
fn d2_words_are_syntax_not_a_second_width_of_payload_identity() {
    // D2:01 CLOSE, D2:11 DOT and unmatched D2:10 OPEN fail closed.
    for unframed in ["01", "11", "10"] {
        assert!(
            parse_canonical_binary(unframed).is_err(),
            "D2 structural word {unframed} must not become a domain payload"
        );
    }
    // D2:10 OPEN + D2:01 CLOSE form exactly one empty structure.
    let framed = parse_canonical_binary("10 01").expect("balanced D2 frame");
    assert_eq!(framed.len(), 1);
    assert!(matches!(
        &framed[0].kind,
        ExprKind::List(items) if items.is_empty()
    ));
    // D2:00 is separator, not a standalone payload value.
    assert!(parse_canonical_binary("00")
        .expect("D2 separator alone is permissible")
        .is_empty());
}
