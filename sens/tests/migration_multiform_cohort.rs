//! #4455 — two top-level D2 forms in one exact physical T5 source.
//! Rust checks transport and syntax boundaries; program-result laws remain Lisp-owned.
use sens::{decode_ternary_program, encode_binary_projection_ternary, open_ternary_program, parse_canonical_binary};

const T5: &[u8] =
    include_bytes!("../../../tests/fixtures/migration-multiform-cohort/two-forms.sens");
const SOURCE: &str =
    include_str!("../../../tests/fixtures/migration-multiform-cohort/two-forms.lisp");
const PROJECTION: &str =
    "10 001 00 000 01 00 10 111 00 10 001 00 000 01 00 10 001 00 000 01 01";

#[test]
fn physical_two_form_source_retains_d2_boundary_and_roundtrips() {
    // Provenance linkage only: this test checks the source is the expected
    // two-form input; it does not assert a Rust language result for either form.
    assert_eq!(SOURCE.lines().count(), 2);
    assert_eq!(
        T5,
        [
            0x63, 0x89, 0x06, 0x89, 0x67, 0x89, 0x63, 0x89, 0x06, 0x89, 0x63, 0x89, 0x06,
            0x8c,
        ]
    );
    let words = decode_ternary_program(T5).expect("one canonical physical T5");
    assert_eq!(words.len(), 21);
    let visible = open_ternary_program(T5).expect("binary program opens");
    assert_eq!(
        visible.split_whitespace().nth(5),
        Some("00"),
        "D2 inter-form separator"
    );
    assert_eq!(visible, PROJECTION);
    assert_eq!(encode_binary_projection_ternary(&visible).unwrap(), T5);

    let forms = parse_canonical_binary(&visible).expect("parse two current-domain forms");
    assert_eq!(forms.len(), 2, "two top-level forms, not one concatenated datum");
}

#[test]
fn invalid_t5_tail_never_becomes_a_second_program() {
    assert!(decode_ternary_program(&[243u8]).is_err());
    let mut bad = T5.to_vec();
    bad.push(242u8);
    assert!(decode_ternary_program(&bad).is_err());
}
