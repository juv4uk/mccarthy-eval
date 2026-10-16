//! #5605: bounded D1 canonical reader / historical compatibility evidence.
//! This is a reader-and-physical-roundtrip test, NOT a SENS COND execution oracle.
//! No new law, no D8/SID8 projection, no Zig semantic registry.

use sens::{
    decode_ternary_words, encode_ternary_words, parse, parse_binary_source_words,
    parse_canonical_binary, parse_canonical_word_sequence, parse_exact_domain_source,
    DomainIdentity, Exactness, ExprKind,
};

const BARE_D1_D2_FRAME: &str = "10 110 00 1 00 0 01";

fn assert_d1(expression: &sens::Expr, expected_bit: u16) {
    let ExprKind::DomainIdentity(DomainIdentity::D1(word)) = &expression.kind else {
        panic!("D1 bit must not turn into numeric, D2 structural, or D3 EMPTY");
    };
    assert_eq!(word.word().packed_bits() as u16, expected_bit);
}

#[test]
fn bare_d1_bits_inside_d2_structure_are_canonical_without_width_tags() {
    let expressions = parse_canonical_binary(BARE_D1_D2_FRAME)
        .expect("canonical binary D2 with two bare exact D1 source words");
    assert_eq!(expressions.len(), 1);
    let ExprKind::List(items) = &expressions[0].kind else {
        panic!("D2 open/close must frame the program");
    };
    assert_eq!(items.len(), 3, "D2 spaces cannot become values");
    let ExprKind::DomainIdentity(DomainIdentity::D3(_)) = items[0].kind else {
        panic!("110 remains a D3 head, without constructing semantics in reader");
    };
    assert_d1(&items[1], 1);
    assert_d1(&items[2], 0);
}

#[test]
fn tagged_d1_is_not_canonical_binary_source() {
    for source in [
        "10 110 00 #d1:1 00 0 01",
        "10 110 00 1 00 #d1:0 01",
    ] {
        assert!(parse_canonical_binary(source).is_err(),
            "canonical reader must reject redundant carrier notation");
        assert!(parse_exact_domain_source(source).is_err(),
            "exact-domain reader must not silently accept a legacy tag");
    }
}

#[test]
fn old_compatibility_reader_still_needs_temporary_d1_projection() {
    let human = parse("1").expect("legacy numeric reader must remain distinct");
    assert!(matches!(human[0].kind, ExprKind::Number(_, Exactness::Exact)));
    let tagged = parse("#d1:1").expect("TEMPORARY compatibility D1 carrier");
    assert_d1(&tagged[0], 1);
    let bare = parse_canonical_binary("1").expect("canonical bare D1");
    assert_d1(&bare[0], 1);
}

#[test]
fn original_d1_identity_survives_t5_word_roundtrip_without_tags() {
    let words: Vec<_> = parse_binary_source_words(BARE_D1_D2_FRAME)
        .expect("canonical width-bearing words")
        .into_iter().map(|t| t.word).collect();
    assert_eq!(words.iter().map(|w| w.width()).collect::<Vec<_>>(),
               vec![2, 3, 2, 1, 2, 1, 2]);
    let physical = encode_ternary_words(&words).expect("mechanical T5 encode");
    let decoded = decode_ternary_words(&physical).expect("mechanical T5 decode");
    assert_eq!(decoded, words, "T5 must preserve full exact widths and bits");
    let parsed = parse_canonical_word_sequence(&decoded)
        .expect("SENS D2 structure reader after T5 physical words");
    let ExprKind::List(items) = &parsed[0].kind else { panic!("D2 list"); };
    assert_d1(&items[1], 1);
    assert_d1(&items[2], 0);
}

#[test]
fn bit_width_mutations_do_not_silently_preserve_d1_identity() {
    // 01 is D2 CLOSE, not D1:1 with a redundant leading zero. It closes
    // the list prematurely, leaving an unmatched 01 at the end.
    assert!(parse_canonical_binary("10 110 00 01 00 0 01").is_err());
    // D3 EMPTY and D1 NO are distinct, even when their packed value is zero.
    let empty = parse_canonical_binary("000").unwrap();
    let ExprKind::List(items) = &empty[0].kind else { panic!("D3 EMPTY is list"); };
    assert!(items.is_empty());
    let no = parse_canonical_binary("0").unwrap();
    assert_d1(&no[0], 0);
}
