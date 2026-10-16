//! Mechanical retirement guard. Rust carries exact domain identifiers;
//! obsolete semantic profiles and truthiness paths must not return here.
//!
//! No Rust-side table is allowed to determine language meaning for a domain.

const CORE: &str = include_str!("../src/eval/special_forms/core.rs");
const ENV: &str = include_str!("../src/environment.rs");
const BOOT: &str = include_str!("../src/bootstrap_measurement.rs");
const LIB: &str = include_str!("../src/lib.rs");
const DOMAIN: &str = include_str!("../src/domain_identity.rs");
const REGISTRY: &str = include_str!("../src/semantic_registry.rs");
const MANIFEST: &str = include_str!("../Cargo.toml");
const EVALUATOR: &str = include_str!("../src/eval/canon.rs");
const COMPILER_ROLE: &str = include_str!("../src/compiler_role.rs");
const SELECTOR_MECHANISM: &str = include_str!("../src/eval/selector_law.rs");

#[test]
fn exact_domain_route_has_no_retired_cond_or_graded_predicate_engine() {
    for forbidden in [
        "fn answer_direction(",
        "fn migration_only_cond_truthy(",
        "fn answer(",
        "CondClauseMode",
        "CoreProfile",
        ".is_truthy()",
        "(query expected-result expression)",
    ] {
        assert!(
            !CORE.contains(forbidden),
            "Rust must not own retired language semantics: {forbidden}"
        );
    }
}

#[test]
fn rust_domain_identity_stays_width_qualified_not_a_second_meaning_table() {
    for rung in 1..=9 {
        assert!(DOMAIN.contains(&format!("D{rung}(")), "missing D{rung} carrier");
    }
    assert!(DOMAIN.contains("BinarySourceWord"));
    assert!(DOMAIN.contains("pub const fn packed_bits"));
}

#[test]
fn legacy_cond_profile_state_does_not_reenter_rust_environment() {
    for source in [ENV, BOOT, LIB] {
        assert!(!source.contains("CondClauseMode"));
        assert!(!source.contains("set_cond_clause_mode"));
        assert!(!source.contains("cond_clause_mode"));
    }
}

#[test]
fn d10_mechanical_width_does_not_add_semantic_dispatch() {
    use sens::domain_ladder::DomainCoordinate;
    assert!(DomainCoordinate::new(10, 0).is_some());
    assert!(DomainCoordinate::new(10, 1023).is_some());
    assert!(DomainCoordinate::new(10, 1024).is_none());
    assert!(DomainCoordinate::new(11, 0).is_none());
}

#[test]
fn flat_sid8_cannot_choose_a_rung_or_call_a_domain_mechanism() {
    assert!(
        !REGISTRY.contains("fn legacy_domain_identity_from_registry_byte("),
        "Rust must not own a flat SID8-to-domain meaning table"
    );
    assert!(
        !EVALUATOR.contains("semantic_registry::legacy_domain_identity_from_registry_byte("),
        "historical SID8 bytes must not be delegated to canonical domain operations"
    );
    assert!(
        REGISTRY.contains("direct_domain_identity_for_surface(name)"),
        "surface projection must originate only in the Lisp-generated domain ladder"
    );
}


#[test]
fn legacy_d5_byte_cannot_mint_a_domain_law() {
    // Rust перевіряє лише походження проєкції, а не власноруч описує закон D5.
    assert!(!REGISTRY.contains("match byte {"));
    assert!(
        !REGISTRY.contains("fn transitional_d5_binding_identity_from_registry_byte("),
        "retired SID8-to-D5 alias must remain absent"
    );
    assert!(
        REGISTRY.contains("fn d5_binding_identity_for_definition("),
        "D5 projections must come from the generated domain binding ledger"
    );
    assert!(REGISTRY.contains("D5_DEFINITION_BINDINGS"));
}

#[test]
fn legacy_host_oracle_schema_is_not_part_of_rust_core() {
    assert!(!MANIFEST.contains("legacy-evidence-schemas"));
    for forbidden in [
        "pub mod compilation_artifact;",
        "pub mod compilation_artifact_producer;",
        "pub mod conformance_oracle;",
        "pub mod fixpoint_checkpoint;",
        "pub mod gpu_admission;",
        "pub mod gpu_oracle;",
        "pub mod program_compiler;",
        "pub mod selfhost_lineage;",
    ] {
        assert!(!LIB.contains(forbidden), "retired Rust-owned oracle returned: {forbidden}");
    }
}

#[test]
fn compiler_role_is_lisp_owned_and_rust_selector_decoding_cannot_claim_it() {
    // Only structural carrier and call into the SENS-owned role law remain.
    assert!(COMPILER_ROLE.contains("compiler_execution_role_from_sens(identity)"));
    assert!(!COMPILER_ROLE.contains("domain_primitive_kind(identity)"));
    assert!(!COMPILER_ROLE.contains("selector_law::compiler_execution_role(identity)"));
    assert!(!SELECTOR_MECHANISM.contains("fn compiler_execution_role("));
}
