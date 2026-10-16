//! Bounded real migration: two historical top-level forms -> one exact physical T5 stream.
use sens::{decode_ternary_program, encode_binary_projection_ternary, open_ternary_program, parse_canonical_binary};

const T5: &[u8] = include_bytes!("../../../tests/fixtures/migration-multiform-cohort-main/two-forms.sens");
const PROJECTION: &str = "10 001 00 000 01 00 10 111 00 10 001 00 000 01 00 10 001 00 000 01 01";

#[test]
fn two_forms_roundtrip_and_retain_d2_form_boundaries() {
    assert_eq!(T5, [0x63, 0x89, 0x06, 0x89, 0x67, 0x89, 0x63, 0x89, 0x06, 0x89, 0x63, 0x89, 0x06, 0x8c]);
    let words = decode_ternary_program(T5).expect("canonical T5");
    assert_eq!(words.len(), 21);
    let visible = open_ternary_program(T5).expect("open current physical T5");
    assert_eq!(visible, PROJECTION);
    assert_eq!(visible.split_whitespace().nth(5), Some("00"));
    assert_eq!(encode_binary_projection_ternary(&visible).unwrap(), T5);
    let forms = parse_canonical_binary(&visible).expect("two current-domain forms");
    assert_eq!(forms.len(), 2);
}

#[test]
fn corrupt_t5_is_rejected() {
    let mut bad = T5.to_vec();
    bad.push(0xf2);
    assert!(decode_ternary_program(&bad).is_err());
}
