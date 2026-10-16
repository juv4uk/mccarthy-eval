//! #3451/#3454 — exact D5 function-description research over qualified D24.
//! The four identities are binary SENS words; Rust transports exact D24 atoms
//! into the actual SENS evaluator and compares its returned observations.
//! This is not a second semantics implementation, decimal shortcut or oracle.

use sens::{
    eval_parsed_expressions, BinaryNumber, Bit5, CoreD5, DomainIdentity, ErrorKind,
    Exactness, Expr, ExprKind, Session, Span, Value,
};
use std::collections::HashSet;

const FUNCTIONS: [u8; 4] = [0b01010, 0b01011, 0b10110, 0b10111];
const D24_ZERO: &str = "000000000000000000000000";
const D24_ONE: &str = "000000000000000000000001";
const D24_TWO: &str = "000000000000000000000010";

fn atom(bits: &str) -> BinaryNumber {
    assert_eq!(bits.len(), 24, "numeric source is exactly D24");
    let word = bits.bytes().map(|bit| match bit {
        b'0' => 0,
        b'1' => 1,
        _ => panic!("D24 numeric transport contains a non-bit"),
    }).collect::<Vec<u8>>();
    BinaryNumber::from_numeric_domain_bits(24, &word)
        .expect("full-width admitted D24 word")
}

fn form(bits: u8, left: ExprKind, right: ExprKind) -> Expr {
    let span = Span::default();
    Expr {
        kind: ExprKind::List(vec![
            Expr {
                kind: ExprKind::DomainIdentity(DomainIdentity::D5(
                    CoreD5::from_word(Bit5::new(bits).expect("D5 exact word")),
                )),
                span,
            },
            Expr { kind: left, span },
            Expr { kind: right, span },
        ].into()),
        span,
    }
}

fn invoke(bits: u8, left: &BinaryNumber, right: &BinaryNumber) -> Result<Value, ErrorKind> {
    let expression = form(bits,
        ExprKind::BinaryNumber(left.clone()),
        ExprKind::BinaryNumber(right.clone()));
    eval_parsed_expressions(&[expression], &mut Session::default())
        .map(|result| result.value)
        .map_err(|error| error.kind)
}

fn signature(left: &BinaryNumber, right: &BinaryNumber) -> Vec<String> {
    FUNCTIONS.iter().map(|&bits|
        invoke(bits, left, right)
            .map(|result| result.to_string())
            .unwrap_or_else(|error| format!("ERR:{error:?}"))
    ).collect()
}

fn distinct_count(values: &[String]) -> usize {
    values.iter().cloned().collect::<HashSet<_>>().len()
}

#[test]
fn unqualified_legacy_numbers_cannot_be_axiom_operands() {
    let expression = form(0b01010,
        ExprKind::Number(1.0, Exactness::Exact),
        ExprKind::Number(1.0, Exactness::Exact));
    let error = eval_parsed_expressions(&[expression], &mut Session::default())
        .expect_err("legacy unqualified Number must not cross the D24+ arithmetic gate");
    assert_eq!(error.kind, ErrorKind::Type);
}

#[test]
fn qualified_primitive_seed_pairs_have_no_single_separating_tuple() {
    let zero = atom(D24_ZERO);
    let one = atom(D24_ONE);
    for a in [&zero, &one] {
        for b in [&zero, &one] {
            let observed = signature(a, b);
            assert!(distinct_count(&observed) < FUNCTIONS.len(),
                "one-tuple unexpectedly separated four D5 words using only D24 seed values: {observed:?}");
        }
    }
}

#[test]
fn language_derived_d24_two_creates_a_single_separating_witness() {
    let one = atom(D24_ONE);
    let evaluated_two = invoke(0b01010, &one, &one)
        .expect("SENS D5:01010 must execute on same-domain D24 words");
    let Value::BinaryNumber(ref two) = evaluated_two
    else { panic!("SENS addition did not retain an exact D24+ numeric carrier"); };
    assert_eq!(two.numeric_domain_width(), Some(24));
    assert_eq!(two.exact_numeric_domain_bits(), atom(D24_TWO).exact_numeric_domain_bits());

    let observed = signature(&one, &two);
    assert_eq!(distinct_count(&observed), FUNCTIONS.len(),
        "the four real SENS D5 outputs must distinguish the derived D24 tuple: {observed:?}");
    // Keep the language observations, including exact NEG and RATIO forms;
    // do not invent decimal names or a Rust table of operation semantics.
    for (identity, output) in FUNCTIONS.iter().zip(&observed) {
        assert!(!output.starts_with("ERR:"),
            "D5:{identity:05b} lacks an admitted D24 outcome: {output}");
    }
}

#[test]
fn bounded_closure_only_gains_separation_after_sens_derives_two() {
    let zero = atom(D24_ZERO);
    let one = atom(D24_ONE);
    let evaluated_two = invoke(0b01010, &one, &one).expect("SENS D24 plus");
    let Value::BinaryNumber(ref two) = evaluated_two
    else { panic!("derived D24 two is not a numeric-domain atom"); };

    let primitive = [&zero, &one];
    let primitive_separators = primitive.iter()
        .flat_map(|a| primitive.iter().map(move |b| (*a, *b)))
        .filter(|(a,b)| distinct_count(&signature(a, b)) == FUNCTIONS.len())
        .collect::<Vec<_>>();
    assert!(primitive_separators.is_empty(),
        "primitive-only D24 words unexpectedly separate all D5 identities: {primitive_separators:?}");

    let closure = [&zero, &one, two];
    let closure_separators = closure.iter()
        .flat_map(|a| closure.iter().map(move |b| (*a, *b)))
        .filter(|(a,b)| distinct_count(&signature(a, b)) == FUNCTIONS.len())
        .collect::<Vec<_>>();
    assert!(closure_separators.contains(&(&one, two)),
        "language-derived D24 two must produce a separating tuple: {closure_separators:?}");
}
