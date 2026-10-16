//! Transport adapter from host/compiler callers into the SENS-written
//! compiler-role law.
//!
//! Semantic direction is intentionally one-way:
//!
//!   exact DomainIdentity
//!     -> SENS compiler nucleus + generated L1-L5 structural projection
//!     -> abstract CompilerExecutionRole
//!
//! This Rust module owns only bootstrap transport, projection integrity checks,
//! SENS evaluation, and representation conversion of the already-derived role.
//! It must never decide identity -> role from domain coordinates.

use crate::{
    canonical_value_sha256_mechanism, domain_identity_shape_mechanism,
    domain_identity_shape_or_empty_mechanism, eval_parsed_expressions, parse_mixed_exact_domain,
    load_core_library, sha256_source, CompilerExecutionRole, CompilerLoweringRole,
    CoreDomainIdentity, DomainIdentity, ErrorKind, Exactness, Expr, ExprKind, LanguageError,
    Session, Span, Value,
};
use std::rc::Rc;

const COMPILER_NUCLEUS_SOURCE: &str = include_str!("../../../lib/compiler-nucleus.lisp");
const LANGUAGE_CONTRACT: &str = include_str!("../../../language-contract.lisp");

pub const COMPILER_ROLE_LAW_REF: &str =
    "lib/compiler-nucleus.lisp:compiler-lowering-role-from-laws";
pub const COMPILER_D3_PROOF_REF: &str = "contracts/bija3-l1-l5-ratification.lisp";
pub const COMPILER_D4_PROOF_REF: &str = "contracts/d4-bootstrap-ratification.lisp";
pub const COMPILER_AUTHORITY_PATH: &str = "language-contract.lisp";
pub const COMPILER_CONTRACT_VERSION: &str = "11.8";

/// SENS-owned semantic input for one current compiler identity.
///
/// This value contains only language authority facts. It deliberately contains
/// no target/backend mechanism identifier and no repository checkout revision;
/// a serializer may attach its exact source revision as transport provenance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompilerSemanticInput {
    pub identity: CoreDomainIdentity,
    pub lowering_role: CompilerLoweringRole,
    pub authority_ref: &'static str,
    pub proof_ref: &'static str,
    pub semantic_status: &'static str,
    pub authority_path: &'static str,
    pub authority_sha256: String,
    pub language_contract_version: &'static str,
}

#[derive(Clone, Debug)]
pub struct CompilerProgramBootstrapBundle {
    pub d3_law: Value,
    pub d4_law: Value,
    pub d3_proof_ref: &'static str,
    pub d4_proof_ref: &'static str,
    pub request_provenance: Value,
    pub authority_path: &'static str,
    pub authority_sha256: String,
    pub language_contract_version: &'static str,
    pub compiler_nucleus_sha256: String,
}


#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedCompilerProgramRequest {
    pub identity: CoreDomainIdentity,
    pub lowering_role: CompilerLoweringRole,
    pub proof_ref: String,
}

#[derive(Clone, Debug)]
pub struct VerifiedCompilerProgramArtifact {
    pub program_wire_sha256: String,
    pub sens_revision: String,
    pub authority_path: String,
    pub authority_sha256: String,
    pub language_contract_version: String,
    pub compiler_nucleus_sha256: String,
    pub semantic_requests_sha256: String,
    pub semantic_requests: Value,
    pub requests: Vec<VerifiedCompilerProgramRequest>,
}
const LAW_PROJECTION: &str =
    include_str!("../../../knowledge/bija3-l1-l5-structure-projection.json");
const LAW_AUTHORITY: &str = include_str!("../../../contracts/bija3-l1-l5-ratification.lisp");
const D4_LAW_PROJECTION: &str =
    include_str!("../../../knowledge/d4-bootstrap-compiler-structure-projection.json");
const D4_LAW_AUTHORITY: &str = include_str!("../../../contracts/d4-bootstrap-ratification.lisp");

const SHAPE_MECHANISM_NAME: &str = "__compiler_domain_shape_mechanism";
const SHAPE_OR_EMPTY_MECHANISM_NAME: &str = "__compiler_domain_shape_or_empty_mechanism";
const PROGRAM_VALUE_NAME: &str = "__compiler_program_data";
const D3_PROOF_VALUE_NAME: &str = "__compiler_d3_proof";
const D4_PROOF_VALUE_NAME: &str = "__compiler_d4_proof";
const PROVENANCE_VALUE_NAME: &str = "__compiler_program_provenance";
const ARTIFACT_PROVENANCE_VALUE_NAME: &str = "__compiler_artifact_provenance";
const PROGRAM_DIGEST_VALUE_NAME: &str = "__compiler_program_wire_sha256";
const VALUE_DIGEST_MECHANISM_NAME: &str = "__compiler_canonical_value_sha256";
const LAW_VALUE_NAME: &str = "__compiler_l1_l5_law";
const D4_LAW_VALUE_NAME: &str = "__compiler_d4_bootstrap_law";

fn invalid_projection(message: impl Into<String>) -> LanguageError {
    LanguageError::new(ErrorKind::InvalidForm, message, Span::default())
}

fn quoted_json_string(source: &str, key: &str) -> Result<String, LanguageError> {
    let marker = format!("\"{key}\": \"");
    let start = source
        .find(&marker)
        .ok_or_else(|| invalid_projection(format!("missing generated law field: {key}")))?
        + marker.len();
    let rest = &source[start..];
    let end = rest
        .find('"')
        .ok_or_else(|| invalid_projection(format!("unterminated generated law field: {key}")))?;
    Ok(rest[..end].to_string())
}

fn projection_width() -> Result<usize, LanguageError> {
    let domain = LAW_PROJECTION
        .find("\"domain\"")
        .ok_or_else(|| invalid_projection("generated law projection has no domain block"))?;
    let rest = &LAW_PROJECTION[domain..];
    let marker = "\"width\": ";
    let start = rest
        .find(marker)
        .ok_or_else(|| invalid_projection("generated law projection has no domain width"))?
        + marker.len();
    let digits = rest[start..]
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    if digits.is_empty() {
        return Err(invalid_projection("generated law domain width is not numeric"));
    }
    digits
        .parse()
        .map_err(|_| invalid_projection("generated law domain width is invalid"))
}

fn projection_width_from(source: &str) -> Result<usize, LanguageError> {
    let domain = source
        .find("\"domain\"")
        .ok_or_else(|| invalid_projection("generated law projection has no domain block"))?;
    let rest = &source[domain..];
    let marker = "\"width\": ";
    let start = rest
        .find(marker)
        .ok_or_else(|| invalid_projection("generated law projection has no domain width"))?
        + marker.len();
    let digits = rest[start..]
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    if digits.is_empty() {
        return Err(invalid_projection("generated law domain width is not numeric"));
    }
    digits
        .parse()
        .map_err(|_| invalid_projection("generated law domain width is invalid"))
}

fn projection_string_array(source: &str, key: &str) -> Result<Vec<String>, LanguageError> {
    let marker = format!("\"{key}\": [");
    let start = source
        .find(&marker)
        .ok_or_else(|| invalid_projection(format!("generated law projection has no {key} array")))?
        + marker.len();
    let tail = &source[start..];
    let end = tail
        .find(']')
        .ok_or_else(|| invalid_projection(format!("generated {key} array is unterminated")))?;
    Ok(tail[..end]
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(|item| item.trim_matches('"').to_string())
        .collect())
}

fn projection_spine() -> Result<Vec<String>, LanguageError> {
    let marker = "\"L5_spine\": [";
    let start = LAW_PROJECTION
        .find(marker)
        .ok_or_else(|| invalid_projection("generated law projection has no L5 spine"))?
        + marker.len();
    let tail = &LAW_PROJECTION[start..];
    let end = tail
        .find(']')
        .ok_or_else(|| invalid_projection("generated L5 spine is unterminated"))?;
    Ok(tail[..end]
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(|item| item.trim_matches('"').to_string())
        .collect())
}

fn sha256_hex(bytes: &[u8]) -> String {
    sha256_source(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}


fn compiler_artifact_list<'a>(
    value: &'a Value,
    context: &str,
) -> Result<Vec<&'a Value>, LanguageError> {
    let mut items = Vec::new();
    let mut cursor = value;
    loop {
        match cursor {
            Value::Nil => return Ok(items),
            Value::Pair(head, tail) => {
                items.push(head.as_ref());
                cursor = tail.as_ref();
            }
            other => {
                return Err(invalid_projection(format!(
                    "{context} must be a proper list, got {other}"
                )));
            }
        }
    }
}

fn compiler_artifact_field<'a>(
    rows: &[&'a Value],
    field: &str,
) -> Result<&'a Value, LanguageError> {
    let mut found = None;
    for row in rows {
        let parts = compiler_artifact_list(row, "compiler artifact field")?;
        if parts.len() != 2 {
            return Err(invalid_projection(
                "compiler artifact fields must contain exactly name and value",
            ));
        }
        let Value::Symbol(name) = parts[0] else {
            return Err(invalid_projection(
                "compiler artifact field name must be a symbol",
            ));
        };
        if name.as_ref() == field {
            if found.is_some() {
                return Err(invalid_projection(format!(
                    "compiler artifact field {field} is duplicated"
                )));
            }
            found = Some(parts[1]);
        }
    }
    found.ok_or_else(|| invalid_projection(format!("compiler artifact field {field} is missing")))
}

fn compiler_artifact_string(value: &Value, field: &str) -> Result<String, LanguageError> {
    match value {
        Value::String(value) => Ok(value.to_string()),
        other => Err(invalid_projection(format!(
            "compiler artifact field {field} must be a string, got {other}"
        ))),
    }
}

fn valid_lower_hex(value: &str, len: usize) -> bool {
    value.len() == len
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn verify_compiler_program_request(
    request: &Value,
    role_session: &mut Session,
) -> Result<VerifiedCompilerProgramRequest, LanguageError> {
    let parts = compiler_artifact_list(request, "compiler semantic request")?;
    if parts.len() != 4 {
        return Err(invalid_projection(
            "compiler semantic request must contain identity, role, proof and provenance",
        ));
    }

    let Value::DomainIdentity(identity) = parts[0] else {
        return Err(invalid_projection(
            "compiler semantic request identity must be exact DomainIdentity",
        ));
    };
    let core = identity.core_operation().ok_or_else(|| {
        invalid_projection("compiler semantic request identity is not a callable Core identity")
    })?;
    let role = decode_language_lowering_role(parts[1])?
        .ok_or_else(|| invalid_projection("compiler semantic request role must not be empty"))?;
    let derived = eval_parsed_expressions(&[language_role_call(core)], role_session)?.value;
    let expected_role = decode_language_lowering_role(&derived)?
        .ok_or_else(|| invalid_projection("compiler semantic request identity has no SENS role"))?;
    if role != expected_role {
        return Err(invalid_projection(
            "compiler semantic request role disagrees with current SENS authority",
        ));
    }

    let proof_ref = compiler_artifact_string(parts[2], "semantic-request proof-ref")?;
    if proof_ref != proof_ref_for_lowering_role(role) {
        return Err(invalid_projection(
            "compiler semantic request proof disagrees with current SENS authority",
        ));
    }

    let provenance = compiler_artifact_list(parts[3], "semantic-request provenance")?;
    if provenance.len() != 3 {
        return Err(invalid_projection(
            "compiler semantic request provenance must contain path, authority digest and contract version",
        ));
    }
    let path = compiler_artifact_string(provenance[0], "semantic-request authority path")?;
    let authority_sha =
        compiler_artifact_string(provenance[1], "semantic-request authority digest")?;
    let contract_version =
        compiler_artifact_string(provenance[2], "semantic-request contract version")?;
    if path != COMPILER_AUTHORITY_PATH
        || authority_sha != sha256_hex(LANGUAGE_CONTRACT.as_bytes())
        || contract_version != COMPILER_CONTRACT_VERSION
    {
        return Err(invalid_projection(
            "compiler semantic request provenance disagrees with current SENS authority",
        ));
    }

    Ok(VerifiedCompilerProgramRequest {
        identity: core,
        lowering_role: role,
        proof_ref,
    })
}

fn authority_sha256_from(projection: &str) -> Result<String, LanguageError> {
    let authority = projection
        .find("\"semantic_authority\"")
        .ok_or_else(|| invalid_projection("generated law projection has no authority provenance"))?;
    quoted_json_string(&projection[authority..], "sha256")
}

fn verify_projection_authority(
    projection: &str,
    authority: &str,
) -> Result<(), LanguageError> {
    let expected_authority_sha = authority_sha256_from(projection)?;
    let actual_authority_sha = sha256_hex(authority.as_bytes());
    if expected_authority_sha != actual_authority_sha {
        return Err(invalid_projection(
            "generated structural projection is stale against its ratified authority",
        ));
    }
    Ok(())
}

fn bit_list(bits: &str, width: usize) -> Result<Value, LanguageError> {
    if bits.len() != width {
        return Err(invalid_projection(format!(
            "generated law bit width mismatch: expected {width}, got {}",
            bits.len()
        )));
    }
    let values = bits
        .bytes()
        .map(|bit| match bit {
            b'0' => Ok(Value::predicate_bit(false)),
            b'1' => Ok(Value::predicate_bit(true)),
            other => Err(invalid_projection(format!(
                "generated law contains non-binary byte: {other}"
            ))),
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Value::list(values))
}

fn compiler_l1_l5_law_value() -> Result<Value, LanguageError> {
    if !LAW_PROJECTION.contains("\"status\": \"generated-projection-only\"") {
        return Err(invalid_projection(
            "compiler law input is not marked generated-projection-only",
        ));
    }
    if !LAW_PROJECTION.contains("\"compiler_role_table\": false") {
        return Err(invalid_projection(
            "generated law projection must forbid a compiler role table",
        ));
    }
    for forbidden in ["SelectorHead", "SelectorTail", "PairConstruct"] {
        if LAW_PROJECTION.contains(forbidden) {
            return Err(invalid_projection(format!(
                "generated structural law projection precomputes compiler role {forbidden}"
            )));
        }
    }

    verify_projection_authority(LAW_PROJECTION, LAW_AUTHORITY)?;

    let width = projection_width()?;
    let empty = quoted_json_string(LAW_PROJECTION, "L1_empty")?;
    let mask = quoted_json_string(LAW_PROJECTION, "L4_dual_xor_mask")?;
    let spine = projection_spine()?;
    if spine.len() != 4 {
        return Err(invalid_projection(format!(
            "generated L5 spine must contain four structural nodes, got {}",
            spine.len()
        )));
    }

    Ok(Value::list([
        Value::Number(width as f64, Exactness::Exact),
        bit_list(&empty, width)?,
        bit_list(&mask, width)?,
        Value::list(
            spine
                .iter()
                .map(|bits| bit_list(bits, width))
                .collect::<Result<Vec<_>, _>>()?,
        ),
    ]))
}

fn compiler_d4_bootstrap_law_value() -> Result<Value, LanguageError> {
    if !D4_LAW_PROJECTION.contains("\"status\": \"generated-projection-only\"") {
        return Err(invalid_projection(
            "D4 compiler law input is not marked generated-projection-only",
        ));
    }
    for required_false in [
        "\"compiler_role_table\": false",
        "\"backend_mechanism_table\": false",
        "\"human_name_routing\": false",
        "\"d8_admission\": false",
    ] {
        if !D4_LAW_PROJECTION.contains(required_false) {
            return Err(invalid_projection(format!(
                "D4 structural projection is missing non-authority guard: {required_false}"
            )));
        }
    }
    for forbidden in [
        "LambdaForm",
        "DefineForm",
        "lambda-form",
        "define-form",
        "CompilerLoweringRole",
    ] {
        if D4_LAW_PROJECTION.contains(forbidden) {
            return Err(invalid_projection(format!(
                "D4 structural projection precomputes compiler role {forbidden}"
            )));
        }
    }

    verify_projection_authority(D4_LAW_PROJECTION, D4_LAW_AUTHORITY)?;

    let width = projection_width_from(D4_LAW_PROJECTION)?;
    if width != 4 {
        return Err(invalid_projection(format!(
            "D4 bootstrap projection must have exact width 4, got {width}"
        )));
    }

    let parent = quoted_json_string(D4_LAW_PROJECTION, "parent_bits")?;
    if parent.len() + 1 != width {
        return Err(invalid_projection(
            "D4 bootstrap parent must be the exact one-bit-shorter fibre prefix",
        ));
    }

    let children = projection_string_array(D4_LAW_PROJECTION, "children")?;
    if children.len() != 2 {
        return Err(invalid_projection(format!(
            "D4 bootstrap fibre must have exactly two ordered children, got {}",
            children.len()
        )));
    }
    for child in &children {
        if child.len() != width || !child.starts_with(&parent) {
            return Err(invalid_projection(
                "D4 bootstrap child does not preserve its exact D3 parent prefix",
            ));
        }
    }

    Ok(Value::list([
        Value::Number(width as f64, Exactness::Exact),
        bit_list(&parent, width - 1)?,
        Value::list(
            children
                .iter()
                .map(|bits| bit_list(bits, width))
                .collect::<Result<Vec<_>, _>>()?,
        ),
    ]))
}

/// Return the verified, representation-only bootstrap values required by the
/// compiled C1 whole-program entry.
///
/// This function does not expose or compute any DomainIdentity -> role mapping.
/// D3/D4 role meaning remains inside the SENS-written compiler nucleus. The
/// bundle contains only already-verified structural-law values, proof
/// references, and immutable provenance bytes needed to install that nucleus
/// in another substrate.
pub fn compiler_program_bootstrap_bundle() -> Result<CompilerProgramBootstrapBundle, LanguageError> {
    let authority_sha256 = sha256_hex(LANGUAGE_CONTRACT.as_bytes());
    Ok(CompilerProgramBootstrapBundle {
        d3_law: compiler_l1_l5_law_value()?,
        d4_law: compiler_d4_bootstrap_law_value()?,
        d3_proof_ref: COMPILER_D3_PROOF_REF,
        d4_proof_ref: COMPILER_D4_PROOF_REF,
        request_provenance: Value::list([
            Value::String(Rc::from(COMPILER_AUTHORITY_PATH)),
            Value::String(Rc::from(authority_sha256.as_str())),
            Value::String(Rc::from(COMPILER_CONTRACT_VERSION)),
        ]),
        authority_path: COMPILER_AUTHORITY_PATH,
        authority_sha256,
        language_contract_version: COMPILER_CONTRACT_VERSION,
        compiler_nucleus_sha256: sha256_hex(COMPILER_NUCLEUS_SOURCE.as_bytes()),
    })
}

fn symbol(name: &str) -> Expr {
    Expr {
        kind: ExprKind::Symbol(Rc::from(name)),
        span: Span::default(),
    }
}

fn language_role_call(identity: CoreDomainIdentity) -> Expr {
    let exact_identity = DomainIdentity::from_source_word(identity.source_word());
    Expr {
        kind: ExprKind::List(Rc::from(
            vec![
                symbol("compiler-lowering-role-from-laws"),
                symbol(SHAPE_MECHANISM_NAME),
                Expr {
                    kind: ExprKind::DomainIdentity(exact_identity),
                    span: Span::default(),
                },
                symbol(LAW_VALUE_NAME),
                symbol(D4_LAW_VALUE_NAME),
            ]
            .into_boxed_slice(),
        )),
        span: Span::default(),
    }
}

fn compiler_program_call() -> Expr {
    Expr {
        kind: ExprKind::List(Rc::from(
            vec![
                symbol("compiler-compile-program"),
                symbol(SHAPE_OR_EMPTY_MECHANISM_NAME),
                symbol(SHAPE_MECHANISM_NAME),
                symbol(PROGRAM_VALUE_NAME),
                symbol(LAW_VALUE_NAME),
                symbol(D4_LAW_VALUE_NAME),
                symbol(D3_PROOF_VALUE_NAME),
                symbol(D4_PROOF_VALUE_NAME),
                symbol(PROVENANCE_VALUE_NAME),
            ]
            .into_boxed_slice(),
        )),
        span: Span::default(),
    }
}

fn eval_compiler_nucleus(session: &mut Session) -> Result<(), LanguageError> {
    let expressions = parse_mixed_exact_domain(COMPILER_NUCLEUS_SOURCE)?;
    eval_parsed_expressions(&expressions, session)?;
    Ok(())
}

fn compiler_program_artifact_call() -> Expr {
    Expr {
        kind: ExprKind::List(Rc::from(
            vec![
                symbol("compiler-compile-program-artifact"),
                symbol(SHAPE_OR_EMPTY_MECHANISM_NAME),
                symbol(SHAPE_MECHANISM_NAME),
                symbol(VALUE_DIGEST_MECHANISM_NAME),
                symbol(PROGRAM_VALUE_NAME),
                symbol(LAW_VALUE_NAME),
                symbol(D4_LAW_VALUE_NAME),
                symbol(D3_PROOF_VALUE_NAME),
                symbol(D4_PROOF_VALUE_NAME),
                symbol(PROVENANCE_VALUE_NAME),
                symbol(ARTIFACT_PROVENANCE_VALUE_NAME),
                symbol(PROGRAM_DIGEST_VALUE_NAME),
            ]
            .into_boxed_slice(),
        )),
        span: Span::default(),
    }
}

fn decode_language_lowering_role(
    value: &Value,
) -> Result<Option<CompilerLoweringRole>, LanguageError> {
    match value {
        Value::Nil => Ok(None),
        Value::Symbol(name) if name.as_ref() == "quote-form" => {
            Ok(Some(CompilerLoweringRole::QuoteForm))
        }
        Value::Symbol(name) if name.as_ref() == "atom-predicate" => {
            Ok(Some(CompilerLoweringRole::AtomPredicate))
        }
        Value::Symbol(name) if name.as_ref() == "selector-tail" => {
            Ok(Some(CompilerLoweringRole::SelectorTail))
        }
        Value::Symbol(name) if name.as_ref() == "selector-head" => {
            Ok(Some(CompilerLoweringRole::SelectorHead))
        }
        Value::Symbol(name) if name.as_ref() == "atom-equality" => {
            Ok(Some(CompilerLoweringRole::AtomEquality))
        }
        Value::Symbol(name) if name.as_ref() == "cond-form" => {
            Ok(Some(CompilerLoweringRole::CondForm))
        }
        Value::Symbol(name) if name.as_ref() == "pair-construct" => {
            Ok(Some(CompilerLoweringRole::PairConstruct))
        }
        Value::Symbol(name) if name.as_ref() == "lambda-form" => {
            Ok(Some(CompilerLoweringRole::LambdaForm))
        }
        Value::Symbol(name) if name.as_ref() == "define-form" => {
            Ok(Some(CompilerLoweringRole::DefineForm))
        }
        Value::Symbol(name) => Err(LanguageError::new(
            ErrorKind::InvalidForm,
            format!("SENS compiler nucleus returned unknown abstract role: {name}"),
            Span::default(),
        )),
        other => Err(LanguageError::new(
            ErrorKind::InvalidForm,
            format!("SENS compiler nucleus returned non-role value: {other}"),
            Span::default(),
        )),
    }
}

/// Derive the complete current compiler lowering role by executing the
/// SENS-owned compiler law over the ratified D3 and D4 structural inputs.
pub fn compiler_lowering_role_from_sens(
    identity: CoreDomainIdentity,
) -> Result<Option<CompilerLoweringRole>, LanguageError> {
    let mut session = Session::default();
    load_core_library(&mut session)?;

    session
        .environment
        .define(SHAPE_MECHANISM_NAME, domain_identity_shape_mechanism());
    session
        .environment
        .define(LAW_VALUE_NAME, compiler_l1_l5_law_value()?);
    session
        .environment
        .define(D4_LAW_VALUE_NAME, compiler_d4_bootstrap_law_value()?);

    eval_compiler_nucleus(&mut session)?;

    let result = eval_parsed_expressions(&[language_role_call(identity)], &mut session)?.value;
    decode_language_lowering_role(&result)
}

fn install_compiler_program_bindings(
    session: &mut Session,
    program: Value,
) -> Result<(), LanguageError> {
    session
        .environment
        .define(SHAPE_MECHANISM_NAME, domain_identity_shape_mechanism());
    session.environment.define(
        SHAPE_OR_EMPTY_MECHANISM_NAME,
        domain_identity_shape_or_empty_mechanism(),
    );
    session
        .environment
        .define(LAW_VALUE_NAME, compiler_l1_l5_law_value()?);
    session
        .environment
        .define(D4_LAW_VALUE_NAME, compiler_d4_bootstrap_law_value()?);
    session.environment.define(PROGRAM_VALUE_NAME, program);
    session.environment.define(
        D3_PROOF_VALUE_NAME,
        Value::String(Rc::from(COMPILER_D3_PROOF_REF)),
    );
    session.environment.define(
        D4_PROOF_VALUE_NAME,
        Value::String(Rc::from(COMPILER_D4_PROOF_REF)),
    );
    session.environment.define(
        PROVENANCE_VALUE_NAME,
        Value::list([
            Value::String(Rc::from(COMPILER_AUTHORITY_PATH)),
            Value::String(Rc::from(sha256_hex(LANGUAGE_CONTRACT.as_bytes()))),
            Value::String(Rc::from(COMPILER_CONTRACT_VERSION)),
        ]),
    );
    Ok(())
}

/// Execute the SENS-written recursive compiler traversal over canonical
/// program-data.  The host adapter supplies representation mechanisms and
/// provenance values once; it does not walk nodes or select compiler roles.
///
/// Return shape is owned by `lib/compiler-nucleus.lisp`:
/// `(D1-success-bit request...)`.
pub fn compiler_program_requests_from_sens(program: Value) -> Result<Value, LanguageError> {
    let mut session = Session::default();
    load_core_library(&mut session)?;
    install_compiler_program_bindings(&mut session, program)?;
    eval_compiler_nucleus(&mut session)?;
    Ok(eval_parsed_expressions(&[compiler_program_call()], &mut session)?.value)
}

/// Compose one backend-neutral whole-program artifact inside SENS.
///
/// `program_wire_sha256` is mechanical transport provenance for the exact
/// canonical SW\\x01 bytes.  The host does not choose, iterate or serialize
/// semantic requests; SENS performs traversal and artifact composition in one
/// call.
pub fn compiler_program_artifact_from_sens(
    program: Value,
    program_wire_sha256: &str,
    sens_revision: &str,
) -> Result<Value, LanguageError> {
    if program_wire_sha256.len() != 64
        || !program_wire_sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(invalid_projection(
            "compiler program wire digest must be exactly 64 lowercase hexadecimal characters",
        ));
    }
    if sens_revision.len() != 40
        || !sens_revision
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(invalid_projection(
            "compiler SENS revision must be exactly 40 lowercase hexadecimal characters",
        ));
    }

    let mut session = Session::default();
    load_core_library(&mut session)?;
    install_compiler_program_bindings(&mut session, program)?;
    session.environment.define(
        VALUE_DIGEST_MECHANISM_NAME,
        canonical_value_sha256_mechanism(),
    );
    session.environment.define(
        PROGRAM_DIGEST_VALUE_NAME,
        Value::String(Rc::from(program_wire_sha256)),
    );
    session.environment.define(
        ARTIFACT_PROVENANCE_VALUE_NAME,
        Value::list([
            Value::String(Rc::from(sens_revision)),
            Value::String(Rc::from(COMPILER_AUTHORITY_PATH)),
            Value::String(Rc::from(sha256_hex(LANGUAGE_CONTRACT.as_bytes()))),
            Value::String(Rc::from(COMPILER_CONTRACT_VERSION)),
            Value::String(Rc::from(sha256_hex(COMPILER_NUCLEUS_SOURCE.as_bytes()))),
        ]),
    );
    eval_compiler_nucleus(&mut session)?;
    Ok(eval_parsed_expressions(&[compiler_program_artifact_call()], &mut session)?.value)
}

/// Verify a decoded SENS whole-program compiler artifact before any backend
/// mechanism binds to it.
///
/// The expected wire digest and SENS revision are supplied by the caller as
/// transport provenance. Schema, authority, request digest, role, proof and
/// request provenance are all checked against current SENS-owned authority.
pub fn verify_compiler_program_artifact_from_sens(
    artifact: &Value,
    expected_program_wire_sha256: &str,
    expected_sens_revision: &str,
) -> Result<VerifiedCompilerProgramArtifact, LanguageError> {
    if !valid_lower_hex(expected_program_wire_sha256, 64) {
        return Err(invalid_projection(
            "expected compiler program wire digest must be exactly 64 lowercase hexadecimal characters",
        ));
    }
    if !valid_lower_hex(expected_sens_revision, 40) {
        return Err(invalid_projection(
            "expected compiler SENS revision must be exactly 40 lowercase hexadecimal characters",
        ));
    }

    let rows = compiler_artifact_list(artifact, "compiler program artifact")?;
    let Some(Value::Symbol(schema)) = rows.first().copied() else {
        return Err(invalid_projection(
            "compiler program artifact must start with a schema symbol",
        ));
    };
    if schema.as_ref() == "compiler-compilation-error/1" {
        return Err(invalid_projection(
            "compiler-compilation-error/1 cannot enter executable lowering",
        ));
    }
    if schema.as_ref() != "compiler-compilation-artifact/1" {
        return Err(invalid_projection(format!(
            "unknown compiler program artifact schema: {schema}"
        )));
    }

    const ALLOWED_FIELDS: [&str; 7] = [
        "artifact-kind",
        "program-wire-sha256",
        "semantic-requests-sha256",
        "authority-provenance",
        "semantic-requests",
        "required-capabilities",
        "artifact-status",
    ];
    // Exactly seven transport/schema slots: position is a local mask bit,
    // never a numeric SENS-domain identity or a re-interpretation of W1–W10.
    let mut seen_fields = 0u8;
    for row in &rows[1..] {
        let parts = compiler_artifact_list(row, "compiler artifact field")?;
        if parts.len() != 2 {
            return Err(invalid_projection(
                "compiler artifact fields must contain exactly name and value",
            ));
        }
        let Value::Symbol(name) = parts[0] else {
            return Err(invalid_projection(
                "compiler artifact field name must be a symbol",
            ));
        };
        let Some(index) = ALLOWED_FIELDS
            .iter()
            .position(|&allowed| allowed == name.as_ref())
        else {
            return Err(invalid_projection(format!(
                "compiler artifact contains unsupported field {name}"
            )));
        };
        let bit = 1u8 << index;
        if seen_fields & bit != 0 {
            return Err(invalid_projection(format!(
                "duplicate compiler artifact field {name}"
            )));
        }
        seen_fields |= bit;
    }

    let kind = compiler_artifact_field(&rows[1..], "artifact-kind")?;
    if !matches!(kind, Value::Symbol(name) if name.as_ref() == "whole-program") {
        return Err(invalid_projection(
            "compiler artifact kind must be whole-program",
        ));
    }
    let status = compiler_artifact_field(&rows[1..], "artifact-status")?;
    if !matches!(status, Value::Symbol(name) if name.as_ref() == "canonical-backend-neutral") {
        return Err(invalid_projection(
            "compiler artifact status must be canonical-backend-neutral",
        ));
    }
    if !matches!(
        compiler_artifact_field(&rows[1..], "required-capabilities")?,
        Value::Nil
    ) {
        return Err(invalid_projection(
            "compiler artifact required-capabilities must be empty",
        ));
    }

    let program_wire_sha256 = compiler_artifact_string(
        compiler_artifact_field(&rows[1..], "program-wire-sha256")?,
        "program-wire-sha256",
    )?;
    if program_wire_sha256 != expected_program_wire_sha256 {
        return Err(invalid_projection(
            "compiler artifact program wire digest does not match caller provenance",
        ));
    }

    let provenance =
        compiler_artifact_list(
            compiler_artifact_field(&rows[1..], "authority-provenance")?,
            "compiler artifact authority provenance",
        )?;
    if provenance.len() != 5 {
        return Err(invalid_projection(
            "compiler artifact authority provenance must contain revision, path, authority digest, contract version and nucleus digest",
        ));
    }
    let sens_revision = compiler_artifact_string(provenance[0], "SENS revision")?;
    let authority_path = compiler_artifact_string(provenance[1], "authority path")?;
    let authority_sha256 = compiler_artifact_string(provenance[2], "authority digest")?;
    let language_contract_version =
        compiler_artifact_string(provenance[3], "language contract version")?;
    let compiler_nucleus_sha256 =
        compiler_artifact_string(provenance[4], "compiler nucleus digest")?;

    if sens_revision != expected_sens_revision
        || authority_path != COMPILER_AUTHORITY_PATH
        || authority_sha256 != sha256_hex(LANGUAGE_CONTRACT.as_bytes())
        || language_contract_version != COMPILER_CONTRACT_VERSION
        || compiler_nucleus_sha256 != sha256_hex(COMPILER_NUCLEUS_SOURCE.as_bytes())
    {
        return Err(invalid_projection(
            "compiler artifact authority provenance disagrees with current SENS authority",
        ));
    }

    let semantic_requests =
        compiler_artifact_field(&rows[1..], "semantic-requests")?.clone();
    let requests = compiler_artifact_list(&semantic_requests, "compiler semantic requests")?;
    if requests.is_empty() {
        return Err(invalid_projection(
            "compiler whole-program artifact must contain semantic requests",
        ));
    }
    let semantic_requests_sha256 = compiler_artifact_string(
        compiler_artifact_field(&rows[1..], "semantic-requests-sha256")?,
        "semantic-requests-sha256",
    )?;
    let canonical_requests = crate::compiler_evidence_canonical_bytes(&semantic_requests)
        .map_err(invalid_projection)?;
    if sha256_hex(&canonical_requests) != semantic_requests_sha256 {
        return Err(invalid_projection(
            "compiler artifact semantic request digest mismatch",
        ));
    }

    let mut role_session = Session::default();
    load_core_library(&mut role_session)?;
    role_session
        .environment
        .define(SHAPE_MECHANISM_NAME, domain_identity_shape_mechanism());
    role_session
        .environment
        .define(LAW_VALUE_NAME, compiler_l1_l5_law_value()?);
    role_session
        .environment
        .define(D4_LAW_VALUE_NAME, compiler_d4_bootstrap_law_value()?);
    eval_compiler_nucleus(&mut role_session)?;

    let verified_requests = requests
        .into_iter()
        .map(|request| verify_compiler_program_request(request, &mut role_session))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(VerifiedCompilerProgramArtifact {
        program_wire_sha256,
        sens_revision,
        authority_path,
        authority_sha256,
        language_contract_version,
        compiler_nucleus_sha256,
        semantic_requests_sha256,
        semantic_requests,
        requests: verified_requests,
    })
}

/// Backward-compatible three-role view used by the already-landed selector/pair
/// compiler bridge. It delegates to the same full SENS-owned role law and never
/// reconstructs identity-to-role meaning in Rust.
/// Produce the canonical proof-carrying compiler semantic input for one exact
/// current identity.
///
/// Role meaning and D3/D4 proof ownership are selected inside SENS. Downstream
/// consumers may verify and bind a private mechanism, but must not reconstruct
/// either fact from coordinates, names, or a legacy callable identity.
fn proof_ref_for_lowering_role(role: CompilerLoweringRole) -> &'static str {
    match role {
        CompilerLoweringRole::LambdaForm | CompilerLoweringRole::DefineForm => {
            COMPILER_D4_PROOF_REF
        }
        CompilerLoweringRole::QuoteForm
        | CompilerLoweringRole::AtomPredicate
        | CompilerLoweringRole::SelectorTail
        | CompilerLoweringRole::SelectorHead
        | CompilerLoweringRole::AtomEquality
        | CompilerLoweringRole::CondForm
        | CompilerLoweringRole::PairConstruct => COMPILER_D3_PROOF_REF,
    }
}

pub fn compiler_semantic_input_from_sens(
    identity: CoreDomainIdentity,
) -> Result<Option<CompilerSemanticInput>, LanguageError> {
    let Some(lowering_role) = compiler_lowering_role_from_sens(identity)? else {
        return Ok(None);
    };

    let proof_ref = proof_ref_for_lowering_role(lowering_role);

    Ok(Some(CompilerSemanticInput {
        identity,
        lowering_role,
        authority_ref: COMPILER_ROLE_LAW_REF,
        proof_ref,
        semantic_status: "current",
        authority_path: COMPILER_AUTHORITY_PATH,
        authority_sha256: sha256_hex(LANGUAGE_CONTRACT.as_bytes()),
        language_contract_version: COMPILER_CONTRACT_VERSION,
    }))
}

pub fn compiler_execution_role_from_sens(
    identity: CoreDomainIdentity,
) -> Result<Option<CompilerExecutionRole>, LanguageError> {
    match compiler_lowering_role_from_sens(identity)? {
        Some(CompilerLoweringRole::SelectorHead) => {
            Ok(Some(CompilerExecutionRole::SelectorHead))
        }
        Some(CompilerLoweringRole::SelectorTail) => {
            Ok(Some(CompilerExecutionRole::SelectorTail))
        }
        Some(CompilerLoweringRole::PairConstruct) => {
            Ok(Some(CompilerExecutionRole::PairConstruct))
        }
        Some(_) | None => Ok(None),
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn d3(raw: u8) -> CoreDomainIdentity {
        CoreDomainIdentity::D3(
            crate::Bija3::from_word(crate::Bit3::new(raw).expect("D3 test word")),
        )
    }

    fn d4(raw: u8) -> CoreDomainIdentity {
        CoreDomainIdentity::D4(
            crate::CoreD4::from_word(crate::Bit4::new(raw).expect("D4 test word")),
        )
    }

    #[test]
    fn compiler_nucleus_loads_each_definition_without_an_opaque_binding_failure() {
        let expressions =
            parse_mixed_exact_domain(COMPILER_NUCLEUS_SOURCE).expect("compiler nucleus parses");
        let mut session = Session::default();
        load_core_library(&mut session).expect("active core loads");
        for (index, expression) in expressions.iter().enumerate() {
            if let Err(error) =
                eval_parsed_expressions(std::slice::from_ref(expression), &mut session)
            {
                panic!(
                    "compiler nucleus form {index} failed: {error:?}; expression={expression:?}"
                );
            }
        }
    }

    #[test]
    fn compiled_driver_bootstrap_bundle_contains_verified_representation_only_inputs() {
        let bundle = compiler_program_bootstrap_bundle().expect("verified compiler bootstrap bundle");

        assert_eq!(bundle.d3_law, compiler_l1_l5_law_value().unwrap());
        assert_eq!(bundle.d4_law, compiler_d4_bootstrap_law_value().unwrap());
        assert_eq!(bundle.d3_proof_ref, COMPILER_D3_PROOF_REF);
        assert_eq!(bundle.d4_proof_ref, COMPILER_D4_PROOF_REF);
        assert_eq!(bundle.authority_path, COMPILER_AUTHORITY_PATH);
        assert_eq!(bundle.language_contract_version, COMPILER_CONTRACT_VERSION);
        assert_eq!(bundle.authority_sha256, sha256_hex(LANGUAGE_CONTRACT.as_bytes()));
        assert_eq!(
            bundle.compiler_nucleus_sha256,
            sha256_hex(COMPILER_NUCLEUS_SOURCE.as_bytes())
        );

        let provenance = list_values(&bundle.request_provenance);
        assert_eq!(provenance.len(), 3);
        assert!(matches!(
            provenance[0],
            Value::String(value) if value.as_ref() == COMPILER_AUTHORITY_PATH
        ));
        assert!(matches!(
            provenance[1],
            Value::String(value) if value.as_ref() == bundle.authority_sha256.as_str()
        ));
        assert!(matches!(
            provenance[2],
            Value::String(value) if value.as_ref() == COMPILER_CONTRACT_VERSION
        ));

        // Ratchet: the bundle is structural/provenance transport only.
        let rendered = format!("{bundle:?}");
        for forbidden in [
            "QuoteForm",
            "AtomPredicate",
            "SelectorTail",
            "SelectorHead",
            "AtomEquality",
            "Conditional",
            "PairConstruct",
            "LambdaForm",
            "DefineForm",
        ] {
            assert!(
                !rendered.contains(forbidden),
                "bootstrap bundle leaked compiler role meaning: {forbidden}"
            );
        }
    }

    #[test]
    fn stale_law_authority_is_a_bootstrap_failure_before_role_execution() {
        let stale_authority = format!("{LAW_AUTHORITY}\n; parity-test-stale-authority");
        let error = verify_projection_authority(LAW_PROJECTION, &stale_authority)
            .expect_err("stale authority must be rejected before SENS role execution");

        assert!(
            error
                .to_string()
                .contains("stale against its ratified authority"),
            "unexpected stale-projection error: {error}"
        );
    }

    #[test]
    fn d4_projection_authority_is_verified_before_role_execution() {
        let stale_authority = format!("{D4_LAW_AUTHORITY}\n; parity-test-stale-authority");
        let error = verify_projection_authority(D4_LAW_PROJECTION, &stale_authority)
            .expect_err("stale D4 authority must be rejected before SENS role execution");

        assert!(
            error
                .to_string()
                .contains("stale against its ratified authority"),
            "unexpected D4 stale-projection error: {error}"
        );
    }

    #[test]
    fn semantic_input_api_owns_law_proof_and_root_authority_facts() {
        let d3_input = compiler_semantic_input_from_sens(d3(0b010))
            .expect("D3 semantic input")
            .expect("ATOM is in compiler closure");
        assert_eq!(d3_input.lowering_role, CompilerLoweringRole::AtomPredicate);
        assert_eq!(d3_input.authority_ref, COMPILER_ROLE_LAW_REF);
        assert_eq!(d3_input.proof_ref, COMPILER_D3_PROOF_REF);
        assert_eq!(d3_input.semantic_status, "current");
        assert_eq!(d3_input.authority_path, "language-contract.lisp");
        assert_eq!(d3_input.authority_sha256.len(), 64);
        assert_eq!(d3_input.language_contract_version, "11.8");

        let d4_input = compiler_semantic_input_from_sens(d4(0b0010))
            .expect("D4 semantic input")
            .expect("LAMBDA is in compiler closure");
        assert_eq!(d4_input.lowering_role, CompilerLoweringRole::LambdaForm);
        assert_eq!(d4_input.authority_ref, COMPILER_ROLE_LAW_REF);
        assert_eq!(d4_input.proof_ref, COMPILER_D4_PROOF_REF);
        assert_eq!(d4_input.authority_sha256, d3_input.authority_sha256);

        assert!(
            compiler_semantic_input_from_sens(d3(0b000))
                .expect("D3 empty transport")
                .is_none()
        );
        let d8 = CoreDomainIdentity::D8(crate::CoreD8::from_word(
            crate::Bit8::new(0b0000_0010).expect("D8 word"),
        ));
        assert!(
            compiler_semantic_input_from_sens(d8)
                .expect("D8 must fail closed as no compiler input")
                .is_none()
        );
    }

    fn list_values(value: &Value) -> Vec<&Value> {
        let mut out = Vec::new();
        let mut cursor = value;
        loop {
            match cursor {
                Value::Nil => return out,
                Value::Pair(head, tail) => {
                    out.push(head.as_ref());
                    cursor = tail.as_ref();
                }
                other => panic!("expected proper list, got {other}"),
            }
        }
    }

    fn exact_value(identity: CoreDomainIdentity) -> Value {
        Value::DomainIdentity(DomainIdentity::from_source_word(identity.source_word()))
    }

    fn expr_program_data(expr: &Expr) -> Value {
        crate::expr_to_exact_program_data(expr)
            .expect("compiler program-data must be exact-domain source-shaped data")
    }

    fn field_value<'a>(artifact: &'a Value, field: &str) -> &'a Value {
        for entry in list_values(artifact).into_iter().skip(1) {
            let parts = list_values(entry);
            if parts.len() == 2
                && matches!(parts[0], Value::Symbol(name) if name.as_ref() == field)
            {
                return parts[1];
            }
        }
        panic!("artifact field {field} not found: {artifact}");
    }

    #[test]
    fn whole_program_artifact_verifier_rejects_duplicate_fields_before_execution() {
        for field in [
            "artifact-kind", "program-wire-sha256", "semantic-requests-sha256",
            "authority-provenance", "semantic-requests", "required-capabilities",
            "artifact-status",
        ] {
            let repeated = Value::Symbol(Rc::from(field));
            let artifact = Value::list([
                Value::Symbol(Rc::from("compiler-compilation-artifact/1")),
                Value::list([repeated.clone(), Value::Nil]),
                Value::list([repeated, Value::Nil]),
            ]);
            let error = verify_compiler_program_artifact_from_sens(
                &artifact,
                &"00".repeat(32),
                "0123456789abcdef0123456789abcdef01234567",
            )
            .expect_err("duplicate authority field must be rejected");
            assert_eq!(error.kind, ErrorKind::InvalidForm);
            assert!(
                error.to_string().contains("duplicate compiler artifact field"),
                "unexpected duplicate-field error for {field}: {error}"
            );
        }

        let unknown = Value::list([
            Value::Symbol(Rc::from("compiler-compilation-artifact/1")),
            Value::list([Value::Symbol(Rc::from("unknown-backend")), Value::Nil]),
        ]);
        assert!(
            verify_compiler_program_artifact_from_sens(
                &unknown,
                &"00".repeat(32),
                "0123456789abcdef0123456789abcdef01234567",
            )
            .unwrap_err().to_string().contains("unsupported field")
        );
    }

    #[test]
    fn whole_program_artifact_verifier_binds_requests_to_current_sens_authority() {
        let parsed = parse_mixed_exact_domain(COMPILER_NUCLEUS_SOURCE).expect("compiler nucleus parses");
        let lowered = crate::lower_program(&parsed);
        let wire = crate::wire_encode_program(&lowered);
        let decoded = crate::wire_decode_program(&wire).expect("canonical SW\\x01 program wire");
        let program = Value::list(decoded.iter().map(expr_program_data));
        let digest = sha256_hex(&wire);
        let revision = "0123456789abcdef0123456789abcdef01234567";

        let artifact = compiler_program_artifact_from_sens(program, &digest, revision)
            .expect("SENS whole-program artifact");
        let verified =
            verify_compiler_program_artifact_from_sens(&artifact, &digest, revision)
                .unwrap_or_else(|original_error| {
                    // Diagnostic transport only: SENS compiles each exact
                    // source form and returns its OWN D1 verdict.  Rust never
                    // assigns identity, role, admission or expected semantics.
                    // The first rejecting witness localizes a real RED.
                    for (index, expression) in decoded.iter().enumerate() {
                        let one_form = Value::list([expr_program_data(expression)]);
                        let verdict = compiler_program_requests_from_sens(one_form)
                            .unwrap_or_else(|error| {
                                panic!(
                                    "SENS compiler witness form={index} cannot execute: {error}; expr={expression:?}; original={original_error}"
                                )
                            });
                        let rows = list_values(&verdict);
                        if rows.first().and_then(|value| value.as_predicate_bit()) != Some(true) {
                            panic!(
                                "SENS rejected first standalone compiler form={index}: {expression:?}; SENS result={verdict}; original={original_error}"
                            );
                        }
                    }
                    panic!(
                        "SENS accepted every standalone form but rejected the complete program: {original_error}"
                    )
                });

        assert_eq!(verified.program_wire_sha256, digest);
        assert_eq!(verified.sens_revision, revision);
        assert_eq!(verified.authority_path, COMPILER_AUTHORITY_PATH);
        assert_eq!(verified.language_contract_version, COMPILER_CONTRACT_VERSION);
        assert!(!verified.requests.is_empty());
        for role in [
            CompilerLoweringRole::QuoteForm,
            CompilerLoweringRole::AtomPredicate,
            CompilerLoweringRole::SelectorTail,
            CompilerLoweringRole::SelectorHead,
            CompilerLoweringRole::AtomEquality,
            CompilerLoweringRole::CondForm,
            CompilerLoweringRole::PairConstruct,
            CompilerLoweringRole::LambdaForm,
            CompilerLoweringRole::DefineForm,
        ] {
            assert!(
                verified.requests.iter().any(|request| request.lowering_role == role),
                "verified whole-program artifact omitted role {role:?}"
            );
        }
        assert_eq!(
            verified.semantic_requests_sha256,
            sha256_hex(
                &crate::compiler_evidence_canonical_bytes(&verified.semantic_requests)
                    .expect("request evidence bytes")
            )
        );
        for request in &verified.requests {
            assert_eq!(
                request.proof_ref,
                proof_ref_for_lowering_role(request.lowering_role)
            );
        }
    }

    #[test]
    fn whole_program_artifact_verifier_rejects_tampering_and_target_smuggling() {
        let parsed = parse_mixed_exact_domain(COMPILER_NUCLEUS_SOURCE).expect("compiler nucleus parses");
        let lowered = crate::lower_program(&parsed);
        let wire = crate::wire_encode_program(&lowered);
        let decoded = crate::wire_decode_program(&wire).expect("canonical SW\\x01 program wire");
        let program = Value::list(decoded.iter().map(expr_program_data));
        let digest = sha256_hex(&wire);
        let revision = "0123456789abcdef0123456789abcdef01234567";
        let artifact = compiler_program_artifact_from_sens(program, &digest, revision)
            .expect("SENS whole-program artifact");

        assert!(
            verify_compiler_program_artifact_from_sens(
                &artifact,
                &"00".repeat(32),
                revision
            )
            .is_err()
        );
        assert!(
            verify_compiler_program_artifact_from_sens(
                &artifact,
                &digest,
                "fedcba9876543210fedcba9876543210fedcba98"
            )
            .is_err()
        );

        let mut rows = list_values(&artifact)
            .into_iter()
            .cloned()
            .collect::<Vec<_>>();
        rows.push(Value::list([
            Value::Symbol(Rc::from("cuda-target")),
            Value::String(Rc::from("sm_61")),
        ]));
        let smuggled = Value::list(rows);
        assert!(
            verify_compiler_program_artifact_from_sens(&smuggled, &digest, revision)
                .is_err()
        );
    }

    #[test]
    fn whole_program_artifact_wraps_real_wire_traversal_inside_sens() {
        let parsed = parse_mixed_exact_domain(COMPILER_NUCLEUS_SOURCE)
            .expect("compiler nucleus parses through exact-domain seam");
        let lowered = crate::lower_program(&parsed);
        let wire = crate::wire_encode_program(&lowered);
        let decoded = crate::wire_decode_program(&wire).expect("canonical SW\\x01 program wire");
        let program = Value::list(decoded.iter().map(expr_program_data));
        let digest = sha256_hex(&wire);
        let revision = "0123456789abcdef0123456789abcdef01234567";

        let artifact = compiler_program_artifact_from_sens(program.clone(), &digest, revision)
            .expect("SENS whole-program artifact");
        let repeated = compiler_program_artifact_from_sens(program, &digest, revision)
            .expect("deterministic repeated SENS whole-program artifact");
        assert_eq!(artifact, repeated);

        let rows = list_values(&artifact);
        assert!(matches!(
            rows.first(),
            Some(Value::Symbol(name)) if name.as_ref() == "compiler-compilation-artifact/1"
        ));
        assert!(matches!(
            field_value(&artifact, "program-wire-sha256"),
            Value::String(found) if found.as_ref() == digest
        ));
        assert!(matches!(
            field_value(&artifact, "artifact-kind"),
            Value::Symbol(found) if found.as_ref() == "whole-program"
        ));
        let provenance = list_values(field_value(&artifact, "authority-provenance"));
        assert_eq!(provenance.len(), 5);
        assert!(matches!(
            provenance[0],
            Value::String(found) if found.as_ref() == revision
        ));
        assert!(matches!(
            provenance[1],
            Value::String(found) if found.as_ref() == COMPILER_AUTHORITY_PATH
        ));
        assert!(matches!(
            provenance[2],
            Value::String(found)
                if found.as_ref() == sha256_hex(LANGUAGE_CONTRACT.as_bytes())
        ));
        assert!(matches!(
            provenance[3],
            Value::String(found) if found.as_ref() == COMPILER_CONTRACT_VERSION
        ));
        assert!(matches!(
            provenance[4],
            Value::String(found)
                if found.as_ref() == sha256_hex(COMPILER_NUCLEUS_SOURCE.as_bytes())
        ));
        assert!(matches!(
            field_value(&artifact, "required-capabilities"),
            Value::Nil
        ));
        assert!(matches!(
            field_value(&artifact, "artifact-status"),
            Value::Symbol(status) if status.as_ref() == "canonical-backend-neutral"
        ));

        let request_value = field_value(&artifact, "semantic-requests");
        let requests = list_values(request_value);
        assert!(!requests.is_empty(), "whole artifact must carry SENS-produced requests");
        let expected_request_digest = match crate::eval::invoke_value(
            &canonical_value_sha256_mechanism(),
            std::slice::from_ref(request_value),
            &crate::Environment::root(),
            Span::default(),
        )
        .expect("representation-only request digest")
        {
            Value::String(ref value) => value.to_string(),
            other => panic!("request digest mechanism returned non-string: {other}"),
        };
        assert!(matches!(
            field_value(&artifact, "semantic-requests-sha256"),
            Value::String(found) if found.as_ref() == expected_request_digest
        ));
        let roles = requests
            .iter()
            .map(|request| {
                let request = list_values(request);
                match request.get(1) {
                    Some(Value::Symbol(role)) => role.to_string(),
                    other => panic!("request has no symbolic role: {other:?}"),
                }
            })
            .collect::<std::collections::HashSet<_>>();
        for role in [
            "quote-form",
            "atom-predicate",
            "selector-tail",
            "selector-head",
            "atom-equality",
            "cond-form",
            "pair-construct",
            "lambda-form",
            "define-form",
        ] {
            assert!(roles.contains(role), "whole artifact omitted role {role}");
        }

        let rendered = artifact.to_string().to_ascii_lowercase();
        for forbidden in ["cuda", "ptx", "sass", "futhark", "graal", "install-target"] {
            assert!(
                !rendered.contains(forbidden),
                "whole artifact leaked backend policy: {forbidden}"
            );
        }
    }

    #[test]
    fn rejected_program_returns_sens_owned_error_artifact() {
        let d8 = CoreDomainIdentity::D8(crate::CoreD8::from_word(
            crate::Bit8::new(0b0000_0010).expect("D8 word"),
        ));
        let program = Value::list([Value::list([exact_value(d8)])]);
        let digest = "00".repeat(32);
        let artifact = compiler_program_artifact_from_sens(
            program,
            &digest,
            "0123456789abcdef0123456789abcdef01234567",
        )
        .expect("semantic rejection is an artifact value, not a host traversal error");
        let rows = list_values(&artifact);
        assert!(matches!(
            rows.first(),
            Some(Value::Symbol(name)) if name.as_ref() == "compiler-compilation-error/1"
        ));
        assert!(matches!(
            field_value(&artifact, "program-wire-sha256"),
            Value::String(found) if found.as_ref() == digest
        ));
    }

    #[test]
    fn whole_program_artifact_rejects_non_digest_transport_metadata() {
        let program = Value::Nil;
        let error = compiler_program_artifact_from_sens(
            program.clone(),
            "not-a-sha",
            "0123456789abcdef0123456789abcdef01234567",
        )
        .expect_err("malformed mechanical provenance must fail before SENS invocation");
        assert!(
            error
                .to_string()
                .contains("64 lowercase hexadecimal characters")
        );

        let error = compiler_program_artifact_from_sens(
            program,
            &"00".repeat(32),
            "not-a-revision",
        )
        .expect_err("malformed SENS revision must fail before SENS invocation");
        assert!(error.to_string().contains("40 lowercase hexadecimal characters"));
    }

    #[test]
    fn sens_program_traversal_is_role_aware_and_quote_shields_domain_data() {
        let lambda = exact_value(d4(0b0010));
        let quote = exact_value(d3(0b001));
        let atom = exact_value(d3(0b010));
        let d8 = CoreDomainIdentity::D8(crate::CoreD8::from_word(
            crate::Bit8::new(0b0000_0010).expect("D8 word"),
        ));

        // Canonical D4 LAMBDA is variadic in its body: parameters followed
        // by one-or-more body expressions. Keep both forms inside the Lambda
        // so this proves variadic traversal and QUOTE shielding together.
        let program = Value::list([Value::list([
            lambda,
            Value::list([Value::Symbol(Rc::from("x"))]),
            Value::list([quote, exact_value(d8)]),
            Value::list([atom, Value::Symbol(Rc::from("x"))]),
        ])]);

        let result = compiler_program_requests_from_sens(program).expect("SENS program traversal");
        let rows = list_values(&result);
        assert_eq!(
            rows[0].as_predicate_bit(),
            Some(true),
            "quoted D8 data must not be traversed as a compiler call"
        );
        assert_eq!(rows.len(), 4, "success bit plus Lambda/Quote/Atom requests");

        let observed_roles = rows[1..]
            .iter()
            .map(|request| {
                let request = list_values(request);
                match request[1] {
                    Value::Symbol(role) => role.to_string(),
                    other => panic!("request role must be symbolic, got {other}"),
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(observed_roles, ["lambda-form", "quote-form", "atom-predicate"]);
    }

    #[test]
    fn same_payload_wrong_domain_with_wrong_shape_fails_closed() {
        // D3:010 is AtomPredicate and one argument is valid.
        let d3_program = Value::list([Value::list([
            exact_value(d3(0b010)),
            Value::Symbol(Rc::from("x")),
        ])]);
        let d3_result =
            compiler_program_requests_from_sens(d3_program).expect("D3 atom traversal");
        let d3_rows = list_values(&d3_result);
        assert_eq!(d3_rows[0].as_predicate_bit(), Some(true));

        // Same numeric payload under width 4 is D4:0010 LambdaForm.
        // Reusing the one-child D3 source shape must fail rather than silently
        // reinterpret the node as a valid lambda request.
        let d4_program = Value::list([Value::list([
            exact_value(d4(0b0010)),
            Value::Symbol(Rc::from("x")),
        ])]);
        let d4_result =
            compiler_program_requests_from_sens(d4_program).expect("normal fail-closed result");
        let d4_rows = list_values(&d4_result);
        assert_eq!(d4_rows.len(), 1);
        assert_eq!(d4_rows[0].as_predicate_bit(), Some(false));
    }

    #[test]
    fn malformed_exact_d3_cond_clause_fails_closed_before_request_emission() {
        let cond = exact_value(d3(0b110));
        let malformed_clause = Value::list([Value::Symbol(Rc::from("test-only"))]);
        let program = Value::list([Value::list([cond, malformed_clause])]);

        let result =
            compiler_program_requests_from_sens(program).expect("normal fail-closed result");
        let rows = list_values(&result);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].as_predicate_bit(), Some(false));
    }

    #[test]
    fn direct_d8_domain_call_fails_closed_in_sens_program_traversal() {
        let d8 = CoreDomainIdentity::D8(crate::CoreD8::from_word(
            crate::Bit8::new(0b0000_0010).expect("D8 word"),
        ));
        let program = Value::list([Value::list([exact_value(d8)])]);

        let result = compiler_program_requests_from_sens(program).expect("normal fail-closed result");
        let rows = list_values(&result);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].as_predicate_bit(), Some(false));
    }
}
