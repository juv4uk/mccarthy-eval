//! #4467: exact physical T5 and D2/D3 word-boundary canary.
//! Selected-value laws are witnessed in SENS, not in Rust assertions.
use sens::{decode_ternary_program, encode_binary_projection_ternary, open_ternary_program};

const T5: &[u8] =
    include_bytes!("../../../tests/fixtures/core1-fourth-domain-canary/fourth.sens");
const VISIBLE: &str = "10 100 00 10 011 00 10 011 00 10 011 00 10 111 00 10 001 00 000 01 00 10 111 00 10 001 00 000 01 00 10 111 00 10 001 00 000 01 00 10 111 00 10 111 00 10 001 00 000 01 00 10 001 00 000 01 01 00 10 001 00 000 01 01 01 01 01 01 01 01 01";
const EXPECTED_PHYSICAL: &[u8] = &[0x66, 0x38, 0x64, 0x89, 0x64, 0x89, 0x64, 0x89, 0x67, 0x89, 0x63, 0x89, 0x06, 0x89, 0x67, 0x89, 0x63, 0x89, 0x06, 0x89, 0x67, 0x89, 0x63, 0x89, 0x06, 0x89, 0x67, 0x89, 0x67, 0x89, 0x63, 0x89, 0x06, 0x89, 0x63, 0x89, 0x06, 0x8c, 0x15, 0xa7, 0x12, 0x3b, 0x2e, 0xb1, 0x8c, 0x2e, 0xb3];

#[test]
fn fourth_domain_canary_preserves_exact_physical_t5_words() {
    assert_eq!(T5, EXPECTED_PHYSICAL);
    let words = decode_ternary_program(T5).expect("canonical physical T5, 0..4 pad trits");
    let visible = open_ternary_program(T5).expect("typed word view");
    assert_eq!(visible, VISIBLE);
    assert_eq!(
        words.iter().map(ToString::to_string).collect::<Vec<_>>().join(" "),
        VISIBLE
    );
    assert_eq!(encode_binary_projection_ternary(&visible).unwrap(), T5);
}

#[test]
fn corrupt_t5_tail_is_rejected_not_read_as_another_program() {
    let mut appended = T5.to_vec();
    appended.push(0xf2);
    assert!(decode_ternary_program(&appended).is_err());
    assert!(decode_ternary_program(&[243]).is_err());
}
