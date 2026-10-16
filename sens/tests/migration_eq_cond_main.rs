//! Historical EQ/COND migration corpus: typed transport only.
//! Old 8-bit executable semantics are archaeology, not Rust-owned truth laws.

use sens::{
    decode_ternary_program, encode_binary_projection_ternary, open_ternary_program,
    render_ternary_words_spaced, TernaryTransportError,
};

const HISTORICAL_SOURCE: &str =
    include_str!("../../../tests/fixtures/migration-eq-cond-cohort-main/eq-cond-select.lisp");
const SELECT_T5: &[u8] =
    include_bytes!("../../../tests/fixtures/migration-eq-cond-cohort-main/eq-cond-select.sens");
const SKIP_T5: &[u8] =
    include_bytes!("../../../tests/fixtures/migration-eq-cond-cohort-main/eq-cond-skip.sens");

#[test]
fn original_eq_cond_images_roundtrip_as_historical_transport_only() {
    for data in [SELECT_T5, SKIP_T5] {
        let words = decode_ternary_program(data).expect("historical T5 must decode");
        let visible = render_ternary_words_spaced(&words);
        assert_eq!(open_ternary_program(data).unwrap(), visible);
        assert_eq!(encode_binary_projection_ternary(&visible).unwrap(), data);
    }
}

#[test]
fn corrupt_physical_and_legacy_text_are_rejected() {
    assert_eq!(
        decode_ternary_program(&[243]),
        Err(TernaryTransportError::InvalidPhysicalByte)
    );
    let mut bad = SELECT_T5.to_vec();
    bad.push(242);
    assert_eq!(decode_ternary_program(&bad), Err(TernaryTransportError::InvalidTail));
    assert!(encode_binary_projection_ternary(HISTORICAL_SOURCE).is_err());
}
