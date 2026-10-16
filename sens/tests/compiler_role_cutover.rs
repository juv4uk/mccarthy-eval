//! sens#3810 — production role/request cutover to SENS-owned structural law.
//!
//! This production witness intentionally does NOT import or call Rust
//! compiler_execution_role. Generated L1/L4/L5 structure is transported as a
//! first-class Value; SENS derives the role and emits the canonical request.
//! Rust role projection remains only in the separate #3809 differential test.

use sens::syntax::{Expr, ExprKind, Span};
use sens::{
    domain_identity_shape_mechanism, eval_parsed_expressions, load_core_library, parse_mixed_exact_domain,
    sha256_source, Bija3, Bit3, Bit4, Bit8, CoreD4, CoreD8, DomainIdentity, Exactness, Session,
    Value,
};
use std::collections::BTreeSet;
use std::rc::Rc;

const NUCLEUS: &str = include_str!("../../../lib/compiler-nucleus.lisp");
const LAW: &str = include_str!("../../../knowledge/bija3-l1-l5-structure-projection.json");
const MECHANISM_NAME: &str = "__compiler_domain_shape_mechanism";
const LAW_NAME: &str = "__compiler_l1_l5_law";
const PROOF_NAME: &str = "__compiler_l1_l5_proof";
const PROVENANCE_NAME: &str = "__compiler_l1_l5_provenance";

#[derive(Debug)]
struct LawProjection {
    width: u8,
    l1_empty: String,
    l4_mask: String,
    l5_spine: Vec<String>,
    owner_ratification: String,
    authority_sha256: String,
    projection_sha256: String,
}

fn quoted_field(text: &str, key: &str) -> String {
    let marker = format!("\"{key}\": \"");
    let start = text
        .find(&marker)
        .unwrap_or_else(|| panic!("missing JSON field {key}"))
        + marker.len();
    let rest = &text[start..];
    let end = rest
        .find('"')
        .unwrap_or_else(|| panic!("unterminated JSON string field {key}"));
    rest[..end].to_string()
}

fn integer_field(text: &str, key: &str) -> u8 {
    let marker = format!("\"{key}\": ");
    let start = text
        .find(&marker)
        .unwrap_or_else(|| panic!("missing JSON integer field {key}"))
        + marker.len();
    let digits = text[start..]
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    digits
        .parse::<u8>()
        .unwrap_or_else(|error| panic!("invalid {key}: {error}"))
}

fn l5_spine(text: &str) -> Vec<String> {
    let marker = "\"L5_spine\": [";
    let start = text.find(marker).expect("missing L5_spine") + marker.len();
    let rest = &text[start..];
    let end = rest.find(']').expect("unterminated L5_spine");
    rest[..end]
        .lines()
        .filter_map(|line| {
            let first = line.find('"')?;
            let tail = &line[first + 1..];
            let second = tail.find('"')?;
            Some(tail[..second].to_string())
        })
        .collect()
}

fn law_projection() -> LawProjection {
    LawProjection {
        width: integer_field(LAW, "width"),
        l1_empty: quoted_field(LAW, "L1_empty"),
        l4_mask: quoted_field(LAW, "L4_dual_xor_mask"),
        l5_spine: l5_spine(LAW),
        owner_ratification: quoted_field(LAW, "owner_ratification"),
        authority_sha256: quoted_field(LAW, "sha256"),
        projection_sha256: quoted_field(LAW, "projection_sha256"),
    }
}

fn span() -> Span {
    Span::default()
}

fn symbol(name: &str) -> Expr {
    Expr {
        kind: ExprKind::Symbol(name.into()),
        span: span(),
    }
}

fn domain(identity: DomainIdentity) -> Expr {
    Expr {
        kind: ExprKind::DomainIdentity(identity),
        span: span(),
    }
}

fn list(items: Vec<Expr>) -> Expr {
    Expr {
        kind: ExprKind::List(Rc::from(items.into_boxed_slice())),
        span: span(),
    }
}

fn d3(raw: u8) -> DomainIdentity {
    DomainIdentity::D3(Bija3::from_word(Bit3::new(raw).expect("D3 word")))
}

fn bit_list(bits: &str) -> Value {
    Value::list(bits.bytes().map(|bit| match bit {
        b'0' => Value::predicate_bit(false),
        b'1' => Value::predicate_bit(true),
        other => panic!("non-binary law projection byte: {other}"),
    }))
}

fn law_value(law: &LawProjection) -> Value {
    assert!(
        LAW.contains("\"status\": \"generated-projection-only\""),
        "compiler law input must remain projection-only"
    );
    assert!(
        LAW.contains("\"compiler_role_table\": false"),
        "generated law data must not contain a compiler role table"
    );

    Value::list([
        Value::Number(law.width as f64, Exactness::Exact),
        bit_list(&law.l1_empty),
        bit_list(&law.l4_mask),
        Value::list(law.l5_spine.iter().map(|bits| bit_list(bits))),
    ])
}

fn sha256_hex(bytes: &[u8]) -> String {
    sha256_source(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn provenance_value(law: &LawProjection) -> Value {
    Value::list([
        Value::String(Rc::from(law.authority_sha256.as_str())),
        Value::String(Rc::from(law.projection_sha256.as_str())),
        Value::String(Rc::from(sha256_hex(NUCLEUS.as_bytes()))),
    ])
}

fn request(identity: DomainIdentity) -> Expr {
    list(vec![
        symbol("compiler-request-from-l1-l5"),
        symbol(MECHANISM_NAME),
        domain(identity),
        symbol(LAW_NAME),
        symbol(PROOF_NAME),
        symbol(PROVENANCE_NAME),
    ])
}

fn session(law: &LawProjection) -> Session {
    let mut session = Session::default();
    load_core_library(&mut session).expect("active core loads");
    session
        .environment
        .define(MECHANISM_NAME, domain_identity_shape_mechanism());
    session.environment.define(LAW_NAME, law_value(law));
    session.environment.define(
        PROOF_NAME,
        Value::String(Rc::from(law.owner_ratification.as_str())),
    );
    session
        .environment
        .define(PROVENANCE_NAME, provenance_value(law));
    let expressions = parse_mixed_exact_domain(NUCLEUS).expect("compiler nucleus parses");
    eval_parsed_expressions(&expressions, &mut session).expect("compiler nucleus loads");
    session
}

fn list_values(value: &Value) -> Option<Vec<&Value>> {
    let mut out = Vec::new();
    let mut cursor = value;
    loop {
        match cursor {
            Value::Nil => return Some(out),
            Value::Pair(head, tail) => {
                out.push(head.as_ref());
                cursor = tail.as_ref();
            }
            _ => return None,
        }
    }
}

#[test]
fn production_request_roles_are_derived_by_sens_from_generated_law_structure() {
    let law = law_projection();
    assert_eq!(law.width, 3);
    assert_eq!(law.l5_spine.len(), 4);

    let mut session = session(&law);
    let mut roles = BTreeSet::new();
    let mut admitted = 0usize;

    for raw in 0u8..=0b111 {
        let identity = d3(raw);
        let result = eval_parsed_expressions(&[request(identity)], &mut session)
            .unwrap_or_else(|error| panic!("D3:{raw:03b} production request failed: {error:?}"))
            .value;

        if matches!(result, Value::Nil) {
            continue;
        }

        admitted += 1;
        let row = list_values(&result).expect("production request is a proper list");
        assert_eq!(row.len(), 4);
        assert_eq!(row[0], &Value::DomainIdentity(identity));
        assert!(
            matches!(row[2], Value::String(value) if value.as_ref() == law.owner_ratification),
            "proof ref must come from generated law projection"
        );

        let role = match row[1] {
            Value::Symbol(ref value) => value.as_ref().to_string(),
            ref other => panic!("role must be an abstract symbol, got {other}"),
        };
        roles.insert(role);

        let prov = list_values(row[3]).expect("provenance is a proper list");
        assert_eq!(prov.len(), 3);
        assert!(
            matches!(prov[0], Value::String(value) if value.as_ref() == law.authority_sha256)
        );
        assert!(
            matches!(prov[1], Value::String(value) if value.as_ref() == law.projection_sha256)
        );
    }

    assert_eq!(admitted, 3, "bounded compiler slice has exactly three roles");
    assert_eq!(
        roles,
        BTreeSet::from([
            "pair-construct".to_string(),
            "selector-head".to_string(),
            "selector-tail".to_string(),
        ])
    );
}

#[test]
fn production_cutover_fails_closed_outside_d3_even_when_shape_is_available() {
    let law = law_projection();
    let mut session = session(&law);

    let d4_same_payload =
        DomainIdentity::D4(CoreD4::from_word(Bit4::new(0b0100).expect("D4 word")));
    let d8_collision =
        DomainIdentity::D8(CoreD8::from_word(Bit8::new(0b0000_0100).expect("D8 word")));

    for identity in [d4_same_payload, d8_collision] {
        let result = eval_parsed_expressions(&[request(identity)], &mut session)
            .expect("non-D3 request must fail closed as a language result")
            .value;
        assert!(matches!(result, Value::Nil));
    }
}

#[test]
fn production_cutover_source_has_no_rust_role_oracle_or_ready_semantic_anchors() {
    assert!(!NUCLEUS.contains("compiler_execution_role"));
    assert!(!NUCLEUS.contains("D3:100"));
    assert!(!NUCLEUS.contains("D3:011"));
    assert!(!NUCLEUS.contains("D3:111"));
    assert!(
        LAW.contains("\"compiler_role_table\": false"),
        "generated law projection must explicitly deny compiler-role authority"
    );
}
