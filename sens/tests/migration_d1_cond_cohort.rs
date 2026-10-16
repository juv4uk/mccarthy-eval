//! #4455: current D1/COND fixture keeps its exact physical T5 representation.
//! Rust is a transport observer here; COND control and result laws belong to Lisp.

use sens::{
    decode_ternary_program, encode_binary_projection_ternary, open_ternary_program,
    render_ternary_words_spaced, TernaryTransportError,
};

const T5: &[u8] =
    include_bytes!("../../../tests/fixtures/migration-d1-cond-cohort/branch.sens");
const SPACED_VIEW: &str =
    include_str!("../../../tests/fixtures/migration-d1-cond-cohort/branch");
const WORDS: &str =
    "10 110 00 10 0 00 10 100 00 000 01 01 00 10 1 00 1 01 01";

#[test]
fn physical_t5_d1_cond_preserves_source_and_transport_views() {
    assert_eq!(T5, [0x67, 0x38, 0x65, 0x15, 0xbf, 0x12, 0x3b, 0x2d, 0xc4, 0xa9, 0xb1, 0xa1]);
    let words = decode_ternary_program(T5).expect("committed file must be canonical T5");
    let current = render_ternary_words_spaced(&words);
    assert_eq!(current, WORDS);
    assert_eq!(SPACED_VIEW, format!("{current}\n"));
    assert_eq!(encode_binary_projection_ternary(SPACED_VIEW).unwrap(), T5);
    assert_eq!(open_ternary_program(T5).unwrap(), WORDS);
    assert_eq!(encode_binary_projection_ternary(WORDS).unwrap(), T5);
}

#[test]
fn corrupt_physical_tail_never_falls_back_to_lisp_text() {
    let mut bad = T5.to_vec();
    bad.push(242u8);
    assert_eq!(
        decode_ternary_program(&bad),
        Err(TernaryTransportError::InvalidTail)
    );
    assert_eq!(
        decode_ternary_program(&[243u8]),
        Err(TernaryTransportError::InvalidPhysicalByte)
    );
    assert!(encode_binary_projection_ternary("(110 (1 1))").is_err());
}
