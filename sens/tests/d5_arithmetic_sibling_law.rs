//! #3003 — D5 arithmetic sibling-law falsifier.
//!
//! This is the full-protocol scope witness.
//! Exact D5 runtime separately proves the binary-value inverse-orientation
//! equations. Here we verify that the entire historical calling protocols
//! contain extra independent deltas, so no global suffix theorem follows.
//! Occupancy is unchanged.

const OWNER: &str = include_str!("../../../knowledge/d5-historical-full-map.json");
const WIDTHS: &str = include_str!("../../../knowledge/exact-width-admitted-corpus.json");
const HISTORY: &str = include_str!("../../../docs/research/2709-lisp15-arithmetic-ledger.json");

fn record<'a>(text: &'a str, anchor: &str, span: usize) -> &'a str {
    let start = text
        .find(anchor)
        .unwrap_or_else(|| panic!("missing anchor: {anchor}"));
    let end = (start + span).min(text.len());
    &text[start..end]
}

#[test]
fn owner_map_has_the_expected_arithmetic_siblings_without_d4_parents() {
    let plus = record(OWNER, r#""coordinate": "01010""#, 320);
    let difference = record(OWNER, r#""coordinate": "01011""#, 320);
    let times = record(OWNER, r#""coordinate": "10010""#, 320);
    let quotient = record(OWNER, r#""coordinate": "10011""#, 320);

    assert!(plus.contains(r#""name": "PLUS""#));
    assert!(difference.contains(r#""name": "DIFFERENCE""#));
    assert!(times.contains(r#""name": "TIMES""#));
    assert!(quotient.contains(r#""name": "QUOTIENT""#));

    let d4_add_prefix = record(WIDTHS, r#""word": "0101""#, 260);
    let d4_mul_prefix = record(WIDTHS, r#""word": "1001""#, 260);

    assert!(d4_add_prefix.contains(r#""status": "unallocated""#));
    assert!(d4_mul_prefix.contains(r#""status": "unallocated""#));
}

#[test]
fn full_historical_protocol_is_more_than_one_orientation_delta() {
    let plus = record(HISTORY, r#""historical_name": "PLUS""#, 1200);
    let difference = record(HISTORY, r#""historical_name": "DIFFERENCE""#, 1200);
    let times = record(HISTORY, r#""historical_name": "TIMES""#, 1200);
    let quotient = record(HISTORY, r#""historical_name": "QUOTIENT""#, 1400);

    assert!(plus.contains(r#""arity": "variadic""#));
    assert!(difference.contains(r#""arity": "2""#));
    assert!(times.contains(r#""arity": "variadic""#));
    assert!(quotient.contains(r#""arity": "2""#));

    assert!(plus.contains("algebraic sum of arguments"));
    assert!(difference.contains(r#""historical_behavior": "x-y""#));
    assert!(times.contains("product of arguments"));
    assert!(quotient.contains("quotient of x and y"));

    // Multiplicative pairing has an additional historical carrier/policy
    // distinction. Do not import Core-Math exact-Q reciprocal semantics.
    assert!(quotient.contains("number-theoretic quotient"));
    assert!(quotient.contains(r#""relation_to_core_math": "comparison-only-no-shared-identity""#));
}

#[test]
fn classification_separates_value_law_from_full_protocol_scope() {
    let result = include_str!("../../../benchmarks/d5-arithmetic-sibling-law/result.json");

    assert!(result.contains(r#""additive_value_semantics_classification": "LOCAL-SIBLING-LAW""#));
    assert!(result.contains(r#""multiplicative_value_semantics_classification": "LOCAL-SIBLING-LAW""#));
    assert!(result.contains(r#""additive_full_protocol_classification": "MULTI-DELTA-NOT-ONE-SIBLING-LAW""#));
    assert!(result.contains(r#""multiplicative_full_protocol_classification": "MULTI-DELTA-NOT-ONE-SIBLING-LAW""#));
    assert!(result.contains(r#""relation_class": "VALUE-LOCAL-LAW_PLUS_FULL-PROTOCOL-COORDINATE-SCOPE""#));
    assert!(result.contains(r#""core_math_inverse_law_transfer": "FORBIDDEN-WITHOUT-BRIDGE""#));
    assert!(result.contains(r#""owner_map_mutation": "NONE""#));
}
