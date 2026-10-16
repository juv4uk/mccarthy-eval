//! Binary-source structural and T5 transport canary.
//! Rust checks exact-width domain words, D2 framing, and physical bytes only;
//! result laws belong to SENS/Lisp-owned witnesses and the physical CLI gate.

use sens::{
    decode_ternary_program, encode_binary_projection_ternary, open_ternary_program,
    parse_canonical_binary, syntax::{Expr, ExprKind},
};

const BINARY_PROGRAM: &str =
    include_str!("../../../examples/binary/d7-first-program.lisp");
const D3_COND_PROGRAM: &str =
    include_str!("../../../examples/binary/d3-cond-program.bits");
const D3_PRIMITIVES_PROGRAM: &str =
    include_str!("../../../examples/binary/d3-primitives-program.bits");
const D5_LABEL_RECURSION_PROGRAM: &str =
    include_str!("../../../examples/binary/d5-label-recursion.bits");
const D5_LABEL_COPY_PROGRAM: &str =
    include_str!("../../../examples/binary/d5-label-copy.bits");
const D5_LABEL_MAP_PROGRAM: &str =
    include_str!("../../../examples/binary/d5-label-map.bits");

fn assert_exact_domain_ast(expression: &Expr) {
    match &expression.kind {
        ExprKind::DomainIdentity(_) => {},
        ExprKind::List(items) => {
            for item in items.iter() { assert_exact_domain_ast(item); }
        }
        ExprKind::Pair(head, tail) => {
            assert_exact_domain_ast(head);
            assert_exact_domain_ast(tail);
        }
        other => panic!("canonical binary reader added a non-domain form: {other:?}"),
    }
}

fn assert_bit_projection(source: &str) {
    assert!(!source.trim().is_empty(), "binary projection must be non-empty");
    assert!(source.bytes().all(|b| b == b'0' || b == b'1' || b.is_ascii_whitespace()),
        "projection may contain only 0/1 words and whitespace");
    let parsed = parse_canonical_binary(source).expect("exact-width binary D2 syntax");
    for expression in &parsed { assert_exact_domain_ast(expression); }
}

#[test]
fn executable_bit_projection_roundtrips_through_physical_t5_without_retyping() {
    assert_bit_projection(BINARY_PROGRAM);
    let physical = encode_binary_projection_ternary(BINARY_PROGRAM)
        .expect("exact-width binary source encodes to physical T5");
    assert!(!physical.is_empty());
    let words = decode_ternary_program(&physical).expect("canonical T5 bytes decode");
    let visible = open_ternary_program(&physical).expect("T5 opens as a bit projection");
    let normalized = BINARY_PROGRAM.split_whitespace().collect::<Vec<_>>().join(" ");
    assert_eq!(visible, normalized, "T5 must preserve each exact-width word");
    assert_eq!(visible, sens::render_ternary_words_spaced(&words));
    assert_eq!(encode_binary_projection_ternary(&visible).unwrap(), physical);
}

#[test]
fn d3_primitive_program_is_only_width_qualified_binary_source() {
    assert_bit_projection(D3_PRIMITIVES_PROGRAM);
    let physical = encode_binary_projection_ternary(D3_PRIMITIVES_PROGRAM)
        .expect("D3 primitive projection encodes as physical T5");
    let words = decode_ternary_program(&physical).expect("D3 T5 source decodes");
    let visible = open_ternary_program(&physical).expect("D3 T5 opens");
    assert_eq!(visible, D3_PRIMITIVES_PROGRAM.split_whitespace().collect::<Vec<_>>().join(" "));
    assert_eq!(visible, sens::render_ternary_words_spaced(&words));
    assert_eq!(encode_binary_projection_ternary(&visible).unwrap(), physical);
}


#[test]
fn d5_label_recursion_keeps_d5_and_d7_coordinates_in_binary_ast() {
    assert_bit_projection(D5_LABEL_RECURSION_PROGRAM);
    let physical = encode_binary_projection_ternary(D5_LABEL_RECURSION_PROGRAM)
        .expect("D5 LABEL source encodes as physical T5");
    let words = decode_ternary_program(&physical).expect("D5 T5 words decode");
    let visible = open_ternary_program(&physical).expect("D5 T5 opens as exact-width words");
    let normalized = D5_LABEL_RECURSION_PROGRAM
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert_eq!(visible, normalized);
    assert_eq!(visible, sens::render_ternary_words_spaced(&words));
    assert_eq!(encode_binary_projection_ternary(&visible).unwrap(), physical);

    let parsed = parse_canonical_binary(&visible)
        .expect("D2 reader preserves D5 LABEL and D7 binding heads");
    assert_eq!(parsed.len(), 1, "LABEL and its recursive application form one D2 call");
    assert_exact_domain_ast(&parsed[0]);
}



#[test]
fn d5_label_list_copy_preserves_exact_d1_and_empty_data_coordinates() {
    assert_bit_projection(D5_LABEL_COPY_PROGRAM);
    let physical = encode_binary_projection_ternary(D5_LABEL_COPY_PROGRAM)
        .expect("D5 recursive list-copy source encodes as physical T5");
    let words = decode_ternary_program(&physical).expect("recursive list-copy T5 decodes");
    let visible = open_ternary_program(&physical).expect("recursive list-copy T5 opens");
    let normalized = D5_LABEL_COPY_PROGRAM
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert_eq!(visible, normalized);
    assert_eq!(visible, sens::render_ternary_words_spaced(&words));
    assert_eq!(encode_binary_projection_ternary(&visible).unwrap(), physical);

    let parsed = parse_canonical_binary(&visible)
        .expect("canonical D2 reader preserves the recursive list-copy program");
    assert_eq!(parsed.len(), 1);
    assert_exact_domain_ast(&parsed[0]);
}



#[test]
fn d5_label_map_preserves_higher_order_d7_binding_coordinates() {
    assert_bit_projection(D5_LABEL_MAP_PROGRAM);
    let physical = encode_binary_projection_ternary(D5_LABEL_MAP_PROGRAM)
        .expect("D5 higher-order map source encodes as physical T5");
    let words = decode_ternary_program(&physical).expect("D5 map T5 decodes");
    let visible = open_ternary_program(&physical).expect("D5 map T5 opens");
    let normalized = D5_LABEL_MAP_PROGRAM
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert_eq!(visible, normalized);
    assert_eq!(visible, sens::render_ternary_words_spaced(&words));
    assert_eq!(encode_binary_projection_ternary(&visible).unwrap(), physical);

    let parsed = parse_canonical_binary(&visible)
        .expect("canonical D2 reader preserves D5 recursion and D7 mapper names");
    assert_eq!(parsed.len(), 1);
    assert_exact_domain_ast(&parsed[0]);
}


#[test]
fn d3_cond_program_preserves_d1_and_d3_widths_in_structural_ast() {
    assert_bit_projection(D3_COND_PROGRAM);
    let physical = encode_binary_projection_ternary(D3_COND_PROGRAM)
        .expect("exact-width COND projection encodes as physical T5");
    let visible = open_ternary_program(&physical).expect("COND T5 opens");
    assert_eq!(visible, D3_COND_PROGRAM.split_whitespace().collect::<Vec<_>>().join(" "));
    let parsed = parse_canonical_binary(&visible).expect("D2 frames binary source");
    assert_eq!(parsed.len(), 1, "the structural source is one D2 form");
    assert_exact_domain_ast(&parsed[0]);
}

#[test]
fn human_language_spellings_never_pass_the_binary_transport_gate() {
    for source in ["(CONS x y)", "(define x 1)", "(100 x)", "10 0011 00 name 01"] {
        assert!(encode_binary_projection_ternary(source).is_err(),
            "human spelling passed as binary source: {source}");
    }
}

#[test]
fn exact_width_payloads_remain_distinct_across_d1_d3_d7_and_d9() {
    let sources = ["1", "001", "0000001", "000000001"];
    let mut coordinates = Vec::new();
    for source in sources {
        let parsed = parse_canonical_binary(source).expect("D1..D9 bit word parses");
        let ExprKind::DomainIdentity(identity) = &parsed[0].kind else {
            panic!("bit projection must remain a domain identity");
        };
        coordinates.push((identity.width(), identity.packed_bits()));
    }
    assert_eq!(coordinates.iter().map(|(_, bits)| *bits).collect::<Vec<_>>(), vec![1; 4]);
    assert_eq!(coordinates.iter().map(|(width, _)| *width).collect::<Vec<_>>(), vec![1, 3, 7, 9]);
    for left in 0..coordinates.len() {
        for right in (left + 1)..coordinates.len() {
            assert_ne!(coordinates[left].0, coordinates[right].0);
        }
    }
    assert!(encode_binary_projection_ternary("0000000001").is_err(),
        "D10 must not be silently truncated into the D1..D9 T5 carrier");
}

#[test]
fn structural_d2_errors_remain_fail_closed_under_physical_t5() {
    for source in ["01", "10"] {
        // Bypass only the D2 grammar gate: encode these exact-width words
        // directly into physical T5 so the decoder sees valid transport but
        // structurally invalid program input.
        let tokens = sens::parse_binary_source_words(source)
            .expect("invalid D2 form is still a valid width-qualified word");
        let words = tokens.into_iter().map(|token| token.word).collect::<Vec<_>>();
        let packed = sens::encode_ternary_words(&words)
            .expect("exact words encode into physical T5 independently of D2 grammar");
        assert!(open_ternary_program(&packed).is_err(),
            "invalid D2 control word unexpectedly opened: {source}");
    }
    assert!(decode_ternary_program(&[243u8]).is_err(), "invalid base-3 byte must fail closed");
    let mut valid = encode_binary_projection_ternary("10 001 00 000 01").unwrap();
    valid.push(242u8);
    assert!(decode_ternary_program(&valid).is_err(), "noncanonical trailer must fail closed");
}
