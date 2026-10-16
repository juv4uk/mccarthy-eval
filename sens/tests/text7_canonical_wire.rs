//! #1877 TEXT7-WIRE-1 — the canonical Text7 transport law, RED-first.
//!
//! Canonical Text identity is the exact seven-bit W7 data stream. These witnesses pin
//! that the machine transport carries those exact cells and never a human
//! spelling (Unicode/UTF-8/layout), that Text7 stays a distinct domain from
//! Number, String and the bare eight-bit Function8 space, and that every
//! malformed shape fails closed rather than masking or transcoding.

use std::cell::RefCell;
use std::rc::Rc;

use sens::{
    encode_binary_frame, decode_binary_frame, encode_text7, render_text7, BinaryFrame, Exactness,
    Text7, Text7Layout, Text7WireError, Value,
};

const LAYOUTS: [Text7Layout; 3] = [
    Text7Layout::Uk,
    Text7Layout::SaDeva,
    Text7Layout::En,
];

fn cells(values: &[u8]) -> Text7 {
    Text7::from_cells(values.to_vec()).expect("test cells are exact UPC-7 cells")
}

#[test]
fn text7_wire_token_round_trips_exact_cells_and_identity() {
    for values in [vec![], vec![0x41], vec![0x00, 0x7F, 0x2E, 0x10]] {
        let text = cells(&values);
        let token = text.to_canonical_wire_token();

        assert!(token.starts_with(Text7::WIRE_TAG), "token: {token}");
        assert_eq!(
            Text7::from_canonical_wire_token(&token).expect("canonical token round-trips"),
            text,
            "exact cells must survive the round trip"
        );
    }
}

#[test]
fn empty_text7_is_distinct_from_structural_empty_list() {
    let empty_text = cells(&[]);
    assert_eq!(empty_text.to_canonical_wire_token(), "#t7:");
    assert_ne!(
        Value::Text7(empty_text).to_canonical_wire_string(),
        Value::Nil.to_canonical_wire_string(),
        "empty Text7 must not collapse into structural ()"
    );
}

#[test]
fn value_level_canonical_wire_uses_the_token_never_a_human_render() {
    let text = cells(&[0x41, 0x42]);
    assert_eq!(Value::Text7(text).to_canonical_wire_string(), "#t7:4142");

    // A human spelling is never accepted back as canonical transport.
    assert_eq!(
        Text7::from_canonical_wire_token("AB"),
        Err(Text7WireError::MissingTag)
    );
}

#[test]
fn text7_transport_stays_distinct_from_number_string_and_function8() {
    let token = cells(&[0x41]).to_canonical_wire_token();

    assert!(!token.starts_with("#q2:"));
    assert!(!token.starts_with("#b"));

    // A String whose visible bytes coincide with the cells is not Text7.
    let as_string = Value::String(Rc::from("A")).to_canonical_wire_string();
    assert_eq!(as_string, "\"A\"");
    assert_ne!(as_string, token);

    // The bare eight-bit Function8 spelling is not Text7 transport either.
    assert_eq!(
        Text7::from_canonical_wire_token("00001100"),
        Err(Text7WireError::MissingTag)
    );
}

#[test]
fn text7_inside_pair_and_vector_round_trips_via_canonical_wire() {
    let text = cells(&[0x41, 0x42]);

    let nested_vector = Value::Vector(Rc::new(RefCell::new(vec![Value::Text7(text.clone())])));
    assert_eq!(nested_vector.to_canonical_wire_string(), "#(#t7:4142)");

    let nested_pair = Value::Pair(Rc::new(Value::Text7(text)), Rc::new(Value::Nil));
    assert_eq!(nested_pair.to_canonical_wire_string(), "(#t7:4142)");
}

#[test]
fn canonical_transport_is_layout_independent_while_human_render_is_not() {
    // Current keyboard coordinate 0x41 has a distinct Ukrainian,
    // Sanskrit and ASCII glyph while retaining exactly the same W7 bits.
    let text = cells(&[0x41]);
    let token = text.to_canonical_wire_token();
    assert_eq!(token, "#t7:41");

    let mut spellings = Vec::new();
    for layout in LAYOUTS {
        let spelling = render_text7(&text, layout)
            .expect("Current W7 cell has a ratified v2 projection");
        assert_ne!(spelling, token, "human layout must not equal canonical wire");
        assert!(
            Text7::from_canonical_wire_token(&spelling).is_err(),
            "human projection cannot be reinterpreted as canonical transport"
        );

        let reencoded = encode_text7(&spelling, layout)
            .expect("layout's own ratified rendering must re-encode");
        assert_eq!(reencoded, text, "exact D7 identity drift in {layout:?}");
        assert_eq!(reencoded.to_canonical_wire_token(), token);
        spellings.push(spelling);
    }
    assert!(
        spellings.iter().any(|spelling| spelling != &spellings[0]),
        "distinct human alphabets must remain projections, not encoded identity"
    );
}

#[test]
fn cells_above_seven_bits_fail_closed_on_every_path() {
    assert!(Text7::from_cells(vec![0x80]).is_err());

    assert_eq!(
        Text7::from_canonical_wire_token("#t7:80"),
        Err(Text7WireError::CellOutOfRange {
            index: 0,
            byte: 0x80
        })
    );
    assert_eq!(
        Text7::from_canonical_wire_token("#t7:4"),
        Err(Text7WireError::OddLength { digits: 1 })
    );
    assert_eq!(
        Text7::from_canonical_wire_token("#t7:4G"),
        Err(Text7WireError::NonHexDigit {
            index: 1,
            byte: b'G'
        })
    );
    // Uppercase hex is a named failure: one cell stream, one canonical spelling.
    assert_eq!(
        Text7::from_canonical_wire_token("#t7:4A"),
        Err(Text7WireError::NonHexDigit {
            index: 1,
            byte: b'A'
        })
    );
}

#[test]
fn fasl_frame_carries_text7_cells_without_utf8_transcoding() {
    // The Control2/FASL frame already types Text7 (CONTROL_ESCAPE + TYPE_TEXT);
    // the exact cell stream survives with no UTF-8/Unicode spelling in the bytes.
    let text = cells(&[0x41, 0x7F]);
    let frame = BinaryFrame::Text(text);
    let bytes = encode_binary_frame(&frame).expect("frame encodes");

    assert_eq!(&bytes[..4], &[1, 1, 1, 0]);

    let (decoded, consumed) = decode_binary_frame(&bytes).expect("frame decodes");
    assert_eq!(decoded, frame);
    assert_eq!(consumed, bytes.len());
}

#[test]
fn existing_number_transport_law_is_unchanged() {
    assert_eq!(
        Value::Number(42.0, Exactness::Exact).to_canonical_wire_string(),
        "#q2:101010/1"
    );
    assert_eq!(
        Text7::from_canonical_wire_token("#q2:101010/1"),
        Err(Text7WireError::MissingTag)
    );
}
