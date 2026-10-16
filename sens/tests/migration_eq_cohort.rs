//! #4455 — historical and uppercase sources retain canonical physical T5.
//! Rust checks transport bytes only; EQ's result law is owned by Lisp.

use sens::{decode_ternary_program, encode_binary_projection_ternary, open_ternary_program};

const HISTORICAL: &[u8] =
    include_bytes!("../../../tests/fixtures/migration-eq-cohort/eq-legacy-sid.sens");
const LISP15: &[u8] =
    include_bytes!("../../../tests/fixtures/migration-eq-cohort/eq-lisp15.sens");
const VISIBLE: &str = "10 101 00 000 00 000 01";
const PHYSICAL: &[u8] = &[0x66, 0x89, 0x06, 0x36, 0xb3];

#[test]
fn both_migrated_equality_programs_preserve_canonical_transport() {
    for physical in [HISTORICAL, LISP15] {
        assert_eq!(physical, PHYSICAL);
        let words = decode_ternary_program(physical).expect("physical canonical T5 decode");
        let visible = open_ternary_program(physical).expect("typed word boundaries");
        assert_eq!(visible, VISIBLE);
        let words_spaced = words
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(" ");
        assert_eq!(words_spaced, VISIBLE);
        assert_eq!(encode_binary_projection_ternary(&visible).unwrap(), physical);
    }
}

#[test]
fn malformed_physical_eq_bytes_never_fall_back_to_text() {
    for physical in [HISTORICAL, LISP15] {
        let mut overpadded = physical.to_vec();
        overpadded.push(0xf2);
        assert!(decode_ternary_program(&overpadded).is_err());
    }
    assert!(decode_ternary_program(&[243]).is_err());
}
