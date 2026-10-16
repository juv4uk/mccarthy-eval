//! #4455: historical selector fixture as exact-width physical T5 data.
//! D4 operation meaning and arity laws are Lisp-owned, not Rust test oracles.
use sens::{decode_ternary_program, encode_binary_projection_ternary, open_ternary_program};

const T5: &[u8] =
    include_bytes!("../../../tests/fixtures/migration-d4-selector-cohort/caar.sens");
const VISIBLE: &str = "10 1000 00 10 111 00 10 111 00 10 001 00 000 01 00 10 001 00 000 01 01 00 10 001 00 000 01 01 01";
const EXPECTED_PHYSICAL: &[u8] = &[0x66, 0x12, 0xc4, 0x7e, 0xc4, 0x7e, 0xc3, 0x2d, 0xa4, 0x2d, 0xc3, 0x2d, 0xa4, 0x2e, 0xa9, 0x37, 0xa8, 0x13, 0xb1, 0xa1];

#[test]
fn historical_selector_fixture_preserves_exact_width_words_and_t5() {
    assert_eq!(T5, EXPECTED_PHYSICAL);
    let words = decode_ternary_program(T5).expect("must decode canonical physical T5");
    let visible = open_ternary_program(T5).expect("retain exact D1..D9 word widths");
    assert_eq!(visible, VISIBLE);
    assert_eq!(
        words.iter().map(ToString::to_string).collect::<Vec<_>>().join(" "),
        VISIBLE
    );
    assert_eq!(encode_binary_projection_ternary(&visible).unwrap(), T5);
}

#[test]
fn corrupted_t5_tail_cannot_be_accepted_as_semantics() {
    let mut tampered = T5.to_vec();
    tampered.push(0xf2);
    assert!(decode_ternary_program(&tampered).is_err());
    assert!(decode_ternary_program(&[243u8]).is_err());
}
