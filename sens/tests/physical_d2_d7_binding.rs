//! Physical T5 D2/D7 contextual binding witness; no ASCII Lisp fallback.
//! D2 owns structure, D7 owns exact Text7 cells, D4 owns LAMBDA.
//! This proves physical T5 can carry D7 contextual identifiers without
//! treating arbitrary untyped D2/W7 data as executable symbols.
use sens::{
    decode_ternary_program, encode_binary_projection_ternary,
    eval_t5_program, open_ternary_program, parse_canonical_binary,
    Session, TernaryTransportError, Value,
};

#[test]
fn existing_d2_d7_and_d4_lambda_execute_from_real_t5_bytes() {
    // ((D4 LAMBDA ((D2-framed D7 'j')) (D2-framed D7 'j')) D3 EMPTY)
    // Structural brackets/separators are D2, not human Lisp punctuation;
    // 1101010 is an exact W7 cell (106), not a host symbol spelling.
    const WORDS: &str =
        "10 10 0010 10 10 1101010 01 01 10 1101010 01 01 000 01";
    let t5 = encode_binary_projection_ternary(WORDS).unwrap();
    assert!(!t5.is_empty());
    let read = decode_ternary_program(&t5).unwrap();
    assert!(!read.is_empty());
    assert_eq!(open_ternary_program(&t5).unwrap(), WORDS);
    let parsed = parse_canonical_binary(WORDS).unwrap();
    assert_eq!(parsed.len(), 1);
    let mut session = Session::default();
    let result = eval_t5_program(&t5, &mut session)
        .expect("D2 structure + D7 contextual binder + D4 lambda must work");
    assert_eq!(result.value, Value::Nil);
}

#[test]
fn canonical_t5_d2_d7_has_no_raw_textual_source_backdoor() {
    // D2/W7 data is valid physical data but isn't automatically a function.
    let data = "10 1101010 01";
    let physical = encode_binary_projection_ternary(data).unwrap();
    assert_eq!(decode_ternary_program(&physical).unwrap().len(), 3);
    assert_eq!(open_ternary_program(&physical).unwrap(), data);
    // The old host Lisp oracle must not be declared canonical by its
    // presence in a D2 structure: nonbinary identifiers stay rejected.
    assert_eq!(
        encode_binary_projection_ternary("(00001001 identity (00001000 (value) value))"),
        Err(TernaryTransportError::InvalidBinaryProjection)
    );
}
