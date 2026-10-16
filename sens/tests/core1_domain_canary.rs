//! #4467 / #4472: physical Core1 fixture transport and D2 syntax boundary.
//! Core1 selected-value laws are proved by the SENS/Lisp-owned witness.
use sens::{
    decode_ternary_program, encode_binary_projection_ternary, open_ternary_program,
    parse_canonical_binary, render_ternary_words_spaced, TernaryTransportError,
};

const T5: &[u8] =
    include_bytes!("../../../tests/fixtures/core1-domain-canary/second.sens");

#[test]
fn physical_core1_second_canary_preserves_exact_t5_words() {
    assert_eq!(T5.len(), 22, "the committed T5 fixture is exactly 22 bytes");
    // The entrypoint is *physical bytes*, not a handcrafted evaluator value.
    let words = decode_ternary_program(T5)
        .expect("canonical T5 and D2 source structure");
    let visible = render_ternary_words_spaced(&words);
    assert_eq!(open_ternary_program(T5).unwrap(), visible);
    assert_eq!(encode_binary_projection_ternary(&visible).unwrap(), T5);

}

#[test]
fn corrupted_or_legacy_transport_cannot_be_admitted_as_the_core1_canary() {
    let mut trailer = T5.to_vec();
    trailer.push(242); // obsolete EOS "22" block, not canonical file EOF
    assert_eq!(
        decode_ternary_program(&trailer),
        Err(TernaryTransportError::InvalidTail)
    );
    assert_eq!(
        decode_ternary_program(&[243u8]),
        Err(TernaryTransportError::InvalidPhysicalByte)
    );
    assert!(encode_binary_projection_ternary("(CAR (CDR X))").is_err());
    assert!(parse_canonical_binary("10 100 00 10 011").is_err());
}
