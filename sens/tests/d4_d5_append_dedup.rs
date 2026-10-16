//! Exact-width D4/D5 carrier tests only; APPEND/CAAR laws belong to SENS/Lisp.
use sens::{decode_ternary_program, encode_binary_projection_ternary, open_ternary_program};

#[test]
fn same_transport_grammar_keeps_d4_and_d5_residents_width_qualified() {
    let cases = [
        ("10 1111 01", 4usize, 0b1111u16),
        ("10 10000 01", 5usize, 0b10000u16),
    ];
    for (source, width, payload) in cases {
        let physical = encode_binary_projection_ternary(source).expect("exact-width D2 words");
        let words = decode_ternary_program(&physical).expect("canonical physical T5");
        assert_eq!(open_ternary_program(&physical).unwrap(), source);
        assert_eq!(encode_binary_projection_ternary(source).unwrap(), physical);
        assert_eq!(words[1].width(), width);
        assert_eq!(words[1].packed_bits(), payload);
    }
}

#[test]
fn same_payload_in_d4_and_d5_cannot_collapse_to_one_coordinate() {
    let d4 = encode_binary_projection_ternary("10 1111 01").unwrap();
    let d5 = encode_binary_projection_ternary("10 01111 01").unwrap();
    assert_ne!(d4, d5);
    let d4_words = decode_ternary_program(&d4).unwrap();
    let d5_words = decode_ternary_program(&d5).unwrap();
    assert_eq!(d4_words[1].packed_bits(), d5_words[1].packed_bits());
    assert_ne!(d4_words[1].width(), d5_words[1].width());
}

#[test]
fn malformed_t5_never_becomes_a_d4_or_d5_operation() {
    assert!(decode_ternary_program(&[243u8]).is_err());
    let mut bad = encode_binary_projection_ternary("10 1111 01").unwrap();
    bad.push(0xf2);
    assert!(decode_ternary_program(&bad).is_err());
}