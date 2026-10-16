//! sens#3805 — static bootstrap firewall for the compiler-in-language path.
//!
//! These checks do not prove compiler semantics. They prevent ownership
//! regression: production host glue may transport/load/execute SENS, but must
//! not regain identity->meaning authority.

const BOUNDARY: &str =
    include_str!("../../../contracts/compiler-bootstrap-boundary-v1.lisp");
const BOOTSTRAP: &str = include_str!("../src/compiler_bootstrap.rs");
const LANGUAGE: &str = include_str!("../src/compiler_language.rs");
const ORACLE: &str = include_str!("../src/compiler_role.rs");
const NUCLEUS: &str = include_str!("../../../lib/compiler-nucleus.lisp");
const CORE1_PRELUDE: &str = include_str!("../../../lib/core1-compiler-prelude.lisp");
const LAW_PROJECTION: &str =
    include_str!("../../../knowledge/bija3-l1-l5-structure-projection.json");

#[test]
fn boundary_inventory_names_current_ownership() {
    for required in [
        "compiler-bootstrap-boundary/1",
        "domain_identity_shape_mechanism",
        "compiler_execution_role_from_sens",
        "differential-reference-only",
        "bounded-production-compiler-semantics",
        "production-current-nucleus . no",
    ] {
        assert!(
            BOUNDARY.contains(required),
            "bootstrap boundary is missing required ownership marker: {required}"
        );
    }
}

#[test]
fn production_host_adapter_cannot_call_rust_role_oracle_or_legacy_identity() {
    assert!(
        !LANGUAGE.contains("compiler_execution_role("),
        "production compiler adapter must execute the SENS law, not call the Rust role oracle"
    );
    for forbidden in ["Sens8", "Sid8", "core1-compiler-prelude"] {
        assert!(
            !LANGUAGE.contains(forbidden),
            "production compiler adapter reintroduced forbidden semantic route: {forbidden}"
        );
    }
    assert!(
        LANGUAGE.contains("compiler_execution_role_from_sens"),
        "production adapter entrypoint must remain explicit"
    );
}

#[test]
fn shape_mechanism_stays_representation_only() {
    for forbidden in [
        "CompilerExecutionRole",
        "SelectorHead",
        "SelectorTail",
        "PairConstruct",
        "compiler_execution_role",
    ] {
        assert!(
            !BOOTSTRAP.contains(forbidden),
            "mechanical DomainIdentity decomposition gained semantic authority: {forbidden}"
        );
    }
}

#[test]
fn generated_law_data_and_nucleus_keep_role_ownership_separate() {
    assert!(
        LAW_PROJECTION.contains("\"compiler_role_table\": false"),
        "generated structural projection must explicitly deny compiler-role authority"
    );
    for forbidden in ["SelectorHead", "SelectorTail", "PairConstruct"] {
        assert!(
            !LAW_PROJECTION.contains(forbidden),
            "generated structural law projection precomputed compiler role {forbidden}"
        );
    }

    assert!(
        NUCLEUS.contains("compiler-role-from-l1-l5"),
        "SENS nucleus must retain the executable role law"
    );
    assert!(
        ORACLE.contains("Differential/bootstrap oracle"),
        "Rust role projection must remain labelled differential/bootstrap only"
    );
}

#[test]
fn historical_core1_compiler_prelude_is_donor_not_current_nucleus() {
    assert!(
        CORE1_PRELUDE.contains("historical"),
        "Core1 compiler prelude must retain explicit historical status"
    );
    assert!(
        !LANGUAGE.contains("core1-compiler-prelude"),
        "current production compiler adapter must not load the historical Core1 prelude"
    );
}
