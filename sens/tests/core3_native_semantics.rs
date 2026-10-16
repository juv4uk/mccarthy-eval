//! Mechanical adapter only: all semantic comparisons, ordering and the
//! fail branch are authored in the binary SENS program below.
use sens::{
    encode_binary_projection_ternary, open_ternary_program, parse_canonical_binary,
    eval_parsed_expressions, Session,
};

const LOGIC: &str =
    include_str!("../../../tests/semantic/core3-executors-d3-law.bits");
const PHYSICAL: &[u8] =
    include_bytes!("../../../tests/semantic/core3-executors-d3-law.sens");

#[test]
fn core3_executor_list_has_sens_owned_binary_verdict() {
    assert!(LOGIC.bytes().all(|byte| matches!(
        byte, b'0' | b'1' | b' ' | b'\n' | b'\r' | b'\t'
    )), "the proof must be written in binary SENS");
    assert_eq!(encode_binary_projection_ternary(LOGIC).unwrap(), PHYSICAL);
    let reopened = open_ternary_program(PHYSICAL).expect("canonical T5");
    assert_eq!(
        reopened.split_whitespace().collect::<Vec<_>>(),
        LOGIC.split_whitespace().collect::<Vec<_>>()
    );
    let forms = parse_canonical_binary(&reopened).expect("ratified D1/D2/D3/D4/D7 frames");
    assert_eq!(forms.len(), 1);
    let mut session = Session::default();
    sens::load_core3_library(&mut session).expect("Core3 library load");
    let answer = eval_parsed_expressions(&forms, &mut session)
        .expect("binary SENS verdict failed (unbound fail branch)").value;
    assert_eq!(answer.as_predicate_bit(), Some(true),
        "only the binary SENS program determines the semantic verdict");
}
