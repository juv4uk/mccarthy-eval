//! #4455 historical EQ/COND corpus: physical T5 transport observer ONLY.
//! Historical executable names and old expected-branch behavior are NOT
//! active semantics. Current PredicateBit/D3 COND laws have separate tests.

use sens::{
    decode_ternary_program, encode_binary_projection_ternary, open_ternary_program,
    render_ternary_words_spaced, TernaryTransportError,
};

const HISTORICAL_SOURCE: &str =
    include_str!("../../../tests/fixtures/migration-eq-cond-cohort/eq-cond-select.lisp");
const SELECT_T5: &[u8] =
    include_bytes!("../../../tests/fixtures/migration-eq-cond-cohort/eq-cond-select.sens");
const SKIP_T5: &[u8] =
    include_bytes!("../../../tests/fixtures/migration-eq-cond-cohort/eq-cond-skip.sens");

#[test]
fn historical_eq_cond_images_preserve_typed_transport_without_evaluating_old_logic() {
    for physical in [SELECT_T5, SKIP_T5] {
        let words = decode_ternary_program(physical).expect("canonical physical T5");
        let visible = render_ternary_words_spaced(&words);
        assert_eq!(open_ternary_program(physical).unwrap(), visible);
        assert_eq!(encode_binary_projection_ternary(&visible).unwrap(), physical);
    }
}

#[test]
fn corrupted_t5_or_historical_text_is_not_current_binary_source() {
    assert_eq!(
        decode_ternary_program(&[243]),
        Err(TernaryTransportError::InvalidPhysicalByte)
    );
    let mut bad = SELECT_T5.to_vec();
    bad.push(242);
    assert_eq!(
        decode_ternary_program(&bad),
        Err(TernaryTransportError::InvalidTail)
    );
    assert!(encode_binary_projection_ternary(HISTORICAL_SOURCE).is_err());
}
