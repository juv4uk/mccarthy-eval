//! #4455 — bounded historical fixture transport through exact D3/D2 words.
use sens::{decode_ternary_program, encode_binary_projection_ternary, open_ternary_program};

const CAR_CDR_T5: &[u8] = include_bytes!("../../../tests/fixtures/migration-pair-cohort/pair-car-cdr.sens");
const CONS_T5: &[u8] = include_bytes!("../../../tests/fixtures/migration-pair-cohort/pair-cons.sens");

#[test]
fn car_cdr_fixture_preserves_exact_current_words_and_physical_t5() {
    let expected = "10 100 00 10 011 00 10 001 00 10 000 00 000 01 01 01 01";
    let words = decode_ternary_program(CAR_CDR_T5).expect("canonical physical T5");
    let visible = open_ternary_program(CAR_CDR_T5).expect("canonical T5 opens");
    assert_eq!(visible, expected);
    assert_eq!(encode_binary_projection_ternary(&visible).unwrap(), CAR_CDR_T5);
    assert_eq!(words.iter().map(ToString::to_string).collect::<Vec<_>>().join(" "), expected);
}

#[test]
fn cons_fixture_preserves_exact_current_words_and_physical_t5() {
    let expected = "10 111 00 10 001 00 000 01 00 10 001 00 000 01 01";
    let words = decode_ternary_program(CONS_T5).expect("canonical physical T5");
    let visible = open_ternary_program(CONS_T5).expect("canonical T5 opens");
    assert_eq!(visible, expected);
    assert_eq!(encode_binary_projection_ternary(&visible).unwrap(), CONS_T5);
    assert_eq!(words.iter().map(ToString::to_string).collect::<Vec<_>>().join(" "), expected);
}

#[test]
fn malformed_physical_t5_fails_closed_for_both_migrated_programs() {
    for payload in [CAR_CDR_T5, CONS_T5] {
        let mut corrupt = payload.to_vec();
        corrupt.push(0xf2);
        assert!(decode_ternary_program(&corrupt).is_err());
    }
}
