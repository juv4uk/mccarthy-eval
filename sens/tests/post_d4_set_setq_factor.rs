//! #2589 — factor historical SET vs SETQ before any width inheritance.
//!
//! Research only. No production mutation operator and no D5/D6 coordinate.
//!
//! Hypothesis:
//!   SETQ = syntax-fixed target + nearest-existing/fail shared-location update
//!   SET  = D4-evaluated target + the same mutation core
//!
//! The witness deliberately separates target acquisition from mutation.  Width
//! pressure, if any, belongs to the shared mutation factor; a historical
//! surface name does not inherit a coordinate merely by composing that factor.

use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Domain {
    Core,
    CoreMath,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TargetName {
    domain: Domain,
    name: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TargetExpr {
    Literal(&'static str),
    Ref(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Cell {
    value: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Store {
    // Frames are ordered nearest -> outermost.
    frames: Vec<BTreeMap<&'static str, usize>>,
    cells: Vec<Cell>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum MutationError {
    MissingBinding(&'static str),
    DomainMismatch,
    MissingTargetReference(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MutationObservation {
    selected_frame: usize,
    selected_cell: usize,
    before_value: &'static str,
    after_value: &'static str,
    frames: Vec<BTreeMap<&'static str, usize>>,
    cells: Vec<Cell>,
}

fn sample_store() -> Store {
    let mut nearest = BTreeMap::new();
    let mut outer = BTreeMap::new();
    outer.insert("x", 0);
    outer.insert("y", 1);
    nearest.insert("z", 2);
    Store {
        frames: vec![nearest, outer],
        cells: vec![
            Cell { value: "X-OLD" },
            Cell { value: "Y-OLD" },
            Cell { value: "Z-OLD" },
        ],
    }
}

fn store_with_shadow() -> Store {
    let mut nearest = BTreeMap::new();
    let mut outer = BTreeMap::new();
    nearest.insert("x", 0);
    outer.insert("x", 1);
    Store {
        frames: vec![nearest, outer],
        cells: vec![Cell { value: "INNER" }, Cell { value: "OUTER" }],
    }
}

// Bounded stand-in for already-admitted ordinary evaluation/name lookup.
// It does not mutate and owns no post-D4 semantics.
fn d4_evaluate_target(
    expr: &TargetExpr,
    env: &BTreeMap<&'static str, TargetName>,
) -> Result<TargetName, MutationError> {
    match expr {
        TargetExpr::Literal(name) => Ok(TargetName {
            domain: Domain::Core,
            name,
        }),
        TargetExpr::Ref(name) => env
            .get(name)
            .cloned()
            .ok_or(MutationError::MissingTargetReference(name)),
    }
}

fn setq_target(name: &'static str) -> TargetName {
    TargetName {
        domain: Domain::Core,
        name,
    }
}

// The shared factor under study.  It knows nothing about SET vs SETQ.
fn nearest_existing_fail_update(
    store: &Store,
    target: &TargetName,
    value: &'static str,
) -> Result<MutationObservation, MutationError> {
    if target.domain != Domain::Core {
        return Err(MutationError::DomainMismatch);
    }

    let mut after = store.clone();
    for (frame_index, frame) in after.frames.iter().enumerate() {
        if let Some(&cell_index) = frame.get(target.name) {
            let before = after.cells[cell_index].value;
            after.cells[cell_index].value = value;
            return Ok(MutationObservation {
                selected_frame: frame_index,
                selected_cell: cell_index,
                before_value: before,
                after_value: value,
                frames: after.frames,
                cells: after.cells,
            });
        }
    }

    Err(MutationError::MissingBinding(target.name))
}

fn setq(
    store: &Store,
    literal_target: &'static str,
    value: &'static str,
) -> Result<MutationObservation, MutationError> {
    nearest_existing_fail_update(store, &setq_target(literal_target), value)
}

fn set(
    store: &Store,
    target_expr: &TargetExpr,
    target_env: &BTreeMap<&'static str, TargetName>,
    value: &'static str,
) -> Result<MutationObservation, MutationError> {
    let target = d4_evaluate_target(target_expr, target_env)?;
    nearest_existing_fail_update(store, &target, value)
}

#[test]
fn set_and_setq_have_identical_mutation_signature_after_target_normalization() {
    let store = sample_store();
    let mut target_env = BTreeMap::new();
    target_env.insert(
        "computed-target",
        TargetName {
            domain: Domain::Core,
            name: "x",
        },
    );

    let via_setq = setq(&store, "x", "NEW").unwrap();
    let via_set = set(
        &store,
        &TargetExpr::Ref("computed-target"),
        &target_env,
        "NEW",
    )
    .unwrap();

    assert_eq!(via_set, via_setq);
    assert_eq!(via_set.selected_frame, 1);
    assert_eq!(via_set.selected_cell, 0);
}

#[test]
fn both_select_the_same_nearest_existing_location() {
    let store = store_with_shadow();
    let mut target_env = BTreeMap::new();
    target_env.insert(
        "computed-target",
        TargetName {
            domain: Domain::Core,
            name: "x",
        },
    );

    let via_setq = setq(&store, "x", "NEW").unwrap();
    let via_set = set(
        &store,
        &TargetExpr::Ref("computed-target"),
        &target_env,
        "NEW",
    )
    .unwrap();

    assert_eq!(via_set, via_setq);
    assert_eq!(via_set.selected_frame, 0);
    assert_eq!(via_set.selected_cell, 0);
    assert_eq!(via_set.cells[0].value, "NEW");
    assert_eq!(via_set.cells[1].value, "OUTER");
}

#[test]
fn both_share_the_same_fail_closed_missing_binding_policy() {
    let store = sample_store();
    let target_env = BTreeMap::new();

    assert_eq!(
        setq(&store, "missing", "NEW"),
        Err(MutationError::MissingBinding("missing"))
    );
    assert_eq!(
        set(
            &store,
            &TargetExpr::Literal("missing"),
            &target_env,
            "NEW"
        ),
        Err(MutationError::MissingBinding("missing"))
    );
}

#[test]
fn acquisition_policy_is_observable_but_separate_from_mutation_core() {
    let store = sample_store();
    let mut target_env = BTreeMap::new();
    target_env.insert(
        "computed-target",
        TargetName {
            domain: Domain::Core,
            name: "y",
        },
    );

    // Same value, different acquired target: the observations differ because
    // acquisition selected a different name, not because the mutation law did.
    let via_setq = setq(&store, "x", "NEW").unwrap();
    let via_set = set(
        &store,
        &TargetExpr::Ref("computed-target"),
        &target_env,
        "NEW",
    )
    .unwrap();

    assert_ne!(via_set.selected_cell, via_setq.selected_cell);
    assert_eq!(via_set.after_value, via_setq.after_value);
}

#[test]
fn missing_computed_target_is_an_acquisition_failure_not_mutation_semantics() {
    let store = sample_store();
    let target_env = BTreeMap::new();

    assert_eq!(
        set(
            &store,
            &TargetExpr::Ref("unknown-target"),
            &target_env,
            "NEW"
        ),
        Err(MutationError::MissingTargetReference("unknown-target"))
    );

    // The mutation core is never reached; the store remains unchanged.
    assert_eq!(store, sample_store());
}

#[test]
fn core_math_factor_cannot_be_smuggled_into_core_mutation_domain() {
    let store = sample_store();
    let cross_domain = TargetName {
        domain: Domain::CoreMath,
        name: "x",
    };

    assert_eq!(
        nearest_existing_fail_update(&store, &cross_domain, "NEW"),
        Err(MutationError::DomainMismatch)
    );
}

#[derive(Debug, PartialEq, Eq)]
struct FactorizationAccounting {
    mutation_factor: &'static str,
    target_acquisition: &'static str,
    set_surface_width: Option<u8>,
    coordinate: Option<&'static str>,
}

fn factorization_accounting() -> FactorizationAccounting {
    FactorizationAccounting {
        mutation_factor: "shared-nearest-existing-fail",
        target_acquisition: "separate-d4-composable",
        set_surface_width: None,
        coordinate: None,
    }
}

#[test]
fn factorization_verdict_is_width_conservative() {
    let accounting = factorization_accounting();
    assert_eq!(
        accounting.mutation_factor,
        "shared-nearest-existing-fail"
    );
    assert_eq!(accounting.target_acquisition, "separate-d4-composable");
    assert_eq!(accounting.set_surface_width, None);
    assert_eq!(accounting.coordinate, None);
}
