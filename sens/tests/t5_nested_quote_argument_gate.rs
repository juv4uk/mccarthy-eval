//! Focused diagnostic for #4774: is nested D3 QUOTE intrinsically broken
//! in the CURRENT physical T5 -> D2 parser -> pure Rust evaluator pipeline?
//!
//! This deliberately does not import the draft #3910 Text7/global-binder law,
//! does not rewrite old source, and cannot admit an original executable file.
//! A PASS narrows #4774's "machine-block-one" failure to global/closure/list
//! binding or a more specific current runtime path, not generic D3 QUOTE.

use sens::{encode_binary_projection_ternary, eval_parsed_expressions,
           open_ternary_program, parse_canonical_binary, Session};

fn current_value(source: &str) -> Result<String, String> {
    // Prove actual FILE T5 physical codec bytes, not English evaluator parsing
    // or an extensionless text shortcut.
    let packed = encode_binary_projection_ternary(source)
        .map_err(|error| format!("T5 encode: {error:?}"))?;
    let visible = open_ternary_program(&packed)
        .map_err(|error| format!("T5 decode: {error:?}"))?;
    if visible != source {
        return Err(format!("exact typed word surface changed: {visible} != {source}"));
    }
    let words = parse_canonical_binary(&visible)
        .map_err(|error| format!("D2 parse: {}", error.render(&visible)))?;
    let result = eval_parsed_expressions(&words, &mut Session::default())
        .map_err(|error| format!("current Rust oracle: {}", error.render(&visible)))?;
    Ok(format!("{}", result.value))
}

#[test]
fn standalone_d3_quote_empty_is_executable_from_physical_t5() {
    let quote_empty = "10 001 00 000 01";
    assert_eq!(current_value(quote_empty).unwrap(), "()");
}

#[test]
fn d3_atom_receives_nested_d3_quote_value_not_raw_call_syntax() {
    // (ATOM (QUOTE ())) and (ATOM ()) must observe exactly the same Nil.
    let with_quote = "10 010 00 10 001 00 000 01 01";
    let direct = "10 010 00 000 01";
    let quoted = current_value(with_quote)
        .unwrap_or_else(|why| panic!("nested D3 QUOTE as ATOM argument failed: {why}"));
    let explicit = current_value(direct).expect("ATOM literal Nil on current Rust");
    assert_eq!(quoted, explicit, "QUOTE should produce Nil as argument");
}

#[test]
fn d3_eq_receives_two_nested_d3_quote_values() {
    // (EQ (QUOTE ()) (QUOTE ())) = (EQ () ()) as two evaluated args.
    let quoted = "10 101 00 10 001 00 000 01 00 10 001 00 000 01 01";
    let direct = "10 101 00 000 00 000 01";
    let with_quotes = current_value(quoted)
        .unwrap_or_else(|why| panic!("nested D3 QUOTE as EQ arguments failed: {why}"));
    let simple = current_value(direct).expect("EQ Nil Nil in current Rust");
    assert_eq!(with_quotes, simple, "EQ argument evaluation changed");
}

#[test]
fn d3_cons_receives_two_nested_d3_quote_values() {
    // (CONS (QUOTE ()) (QUOTE ())) = (CONS () ()) in the same oracle.
    let quoted = "10 111 00 10 001 00 000 01 00 10 001 00 000 01 01";
    let direct = "10 111 00 000 00 000 01";
    let with_quotes = current_value(quoted)
        .unwrap_or_else(|why| panic!("nested D3 QUOTE as CONS arguments failed: {why}"));
    let simple = current_value(direct).expect("CONS Nil Nil in current Rust");
    assert_eq!(with_quotes, simple, "CONS argument evaluation changed");
}

#[test]
fn malformed_nested_d2_close_is_never_reclassified_as_quote_value() {
    // Genuine D2 structural law: opening inner QUOTE form must be closed.
    let malformed = "10 010 00 10 001 00 000 01"; // outer OPEN not closed
    assert!(current_value(malformed).is_err());
}
