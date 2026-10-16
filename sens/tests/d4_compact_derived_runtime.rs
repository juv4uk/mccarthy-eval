//! D4 exact-width carrier and physical T5 transport checks only.
//! LIST/APPEND semantics and resident admission are SENS/Lisp-owned laws.

use sens::{decode_ternary_program, encode_binary_projection_ternary, open_ternary_program};

#[test]
fn width_qualified_words_roundtrip_without_borrowing_cross_domain_meaning() {
    let cases = [
        ("10 1110 01", 4usize, 0b1110u16),
        ("10 1111 01", 4usize, 0b1111u16),
        ("10 01110 01", 5usize, 0b01110u16),
        ("10 00001110 01", 8usize, 0b00001110u16),
    ];

    for (source, expected_width, expected_payload) in cases {
        let physical =
            encode_binary_projection_ternary(source).expect("well-formed exact-width D2 source");
        let words = decode_ternary_program(&physical).expect("canonical physical T5");
        assert_eq!(open_ternary_program(&physical).unwrap(), source);
        assert_eq!(encode_binary_projection_ternary(source).unwrap(), physical);

        let resident = words.get(1).expect("D2 open precedes exact-width head");
        assert_eq!(resident.width(), expected_width);
        assert_eq!(resident.packed_bits(), expected_payload);
    }
}

#[test]
fn physical_t5_rejects_invalid_bytes_and_noncanonical_trailers() {
    for source in ["10 1110 01", "10 1111 01", "10 01110 01"] {
        let physical = encode_binary_projection_ternary(source).unwrap();
        assert!(decode_ternary_program(&physical).is_ok());

        let mut padded = physical.to_vec();
        padded.push(0xf2);
        assert!(decode_ternary_program(&padded).is_err());
    }
    assert!(decode_ternary_program(&[243u8]).is_err());
}

#[test]
fn exact_width_word_identity_is_preserved_in_the_visible_projection() {
    let d4 = encode_binary_projection_ternary("10 1110 01").unwrap();
    let d5 = encode_binary_projection_ternary("10 01110 01").unwrap();
    assert_ne!(d4, d5);

    let d4_words = decode_ternary_program(&d4).unwrap();
    let d5_words = decode_ternary_program(&d5).unwrap();
    assert_eq!(d4_words[1].packed_bits(), d5_words[1].packed_bits());
    assert_ne!(d4_words[1].width(), d5_words[1].width());
}
