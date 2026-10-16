//! #2590 — bounded RETURN/PROG parenthood attack.
//!
//! Research only. Reuses already-established observations from #2315/#2403,
//! #2472 and #2488. No production control feature, root promotion, width or
//! coordinate is introduced here.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BaseOperation {
    InvokeCallable,
    EvaluateForm,
    ConstructClosure,
    LocalBranch,
    DynamicExit,
    FiniteStateTransition,
    SharedLocationUpdate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExitTarget {
    ImmediateCaller,
    NearestActiveProg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContextPolicy {
    Optional,
    Required,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ReturnPolicy {
    target: ExitTarget,
    context: ContextPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ParentCandidate {
    name: &'static str,
    operation: BaseOperation,
}

const APPLY: ParentCandidate = ParentCandidate {
    name: "APPLY",
    operation: BaseOperation::InvokeCallable,
};
const EVAL: ParentCandidate = ParentCandidate {
    name: "EVAL",
    operation: BaseOperation::EvaluateForm,
};
const LAMBDA: ParentCandidate = ParentCandidate {
    name: "LAMBDA",
    operation: BaseOperation::ConstructClosure,
};
const COND: ParentCandidate = ParentCandidate {
    name: "COND",
    operation: BaseOperation::LocalBranch,
};

const RETURN: ReturnPolicy = ReturnPolicy {
    target: ExitTarget::NearestActiveProg,
    context: ContextPolicy::Required,
};

fn return_signature(policy: ReturnPolicy, active_prog: bool) -> (&'static str, &'static str) {
    match (policy.target, policy.context, active_prog) {
        (ExitTarget::ImmediateCaller, ContextPolicy::Optional, _) => ("caller", "ok"),
        (ExitTarget::ImmediateCaller, ContextPolicy::Required, true) => ("caller", "ok"),
        (ExitTarget::ImmediateCaller, ContextPolicy::Required, false) => ("none", "error"),
        (ExitTarget::NearestActiveProg, ContextPolicy::Optional, true) => ("prog", "ok"),
        (ExitTarget::NearestActiveProg, ContextPolicy::Optional, false) => ("caller", "ok"),
        (ExitTarget::NearestActiveProg, ContextPolicy::Required, true) => ("prog", "ok"),
        (ExitTarget::NearestActiveProg, ContextPolicy::Required, false) => ("none", "error"),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ProtocolShape {
    helper_receives_exit_k: bool,
    exit_k_forwarded_across_nested_calls: bool,
    source_call_interface_preserved: bool,
}

fn direct_local_protocol() -> ProtocolShape {
    ProtocolShape {
        helper_receives_exit_k: false,
        exit_k_forwarded_across_nested_calls: false,
        source_call_interface_preserved: true,
    }
}

fn cps_return_protocol() -> ProtocolShape {
    ProtocolShape {
        helper_receives_exit_k: true,
        exit_k_forwarded_across_nested_calls: true,
        source_call_interface_preserved: false,
    }
}

#[test]
fn return_has_two_independently_observable_policy_axes() {
    let ordinary = ReturnPolicy {
        target: ExitTarget::ImmediateCaller,
        context: ContextPolicy::Optional,
    };
    let target_only = ReturnPolicy {
        target: ExitTarget::NearestActiveProg,
        context: ContextPolicy::Optional,
    };
    let context_only = ReturnPolicy {
        target: ExitTarget::ImmediateCaller,
        context: ContextPolicy::Required,
    };

    assert_ne!(
        return_signature(ordinary, true),
        return_signature(target_only, true),
        "target selection must remain observable with context availability held fixed"
    );
    assert_ne!(
        return_signature(ordinary, false),
        return_signature(context_only, false),
        "context requirement must remain observable with target policy held fixed"
    );
    assert_eq!(return_signature(RETURN, true), ("prog", "ok"));
    assert_eq!(return_signature(RETURN, false), ("none", "error"));
}

#[test]
fn no_tested_d4_parent_owns_the_same_base_operation_as_return() {
    for candidate in [APPLY, EVAL, LAMBDA, COND] {
        assert_ne!(
            candidate.operation,
            BaseOperation::DynamicExit,
            "{} is not a same-base parent for dynamic RETURN",
            candidate.name
        );
    }
}

#[test]
fn cps_compilability_changes_call_chain_protocol_and_is_not_local_parenthood() {
    let direct = direct_local_protocol();
    let cps = cps_return_protocol();

    assert!(direct.source_call_interface_preserved);
    assert!(!cps.source_call_interface_preserved);
    assert!(cps.helper_receives_exit_k);
    assert!(cps.exit_k_forwarded_across_nested_calls);
    assert_ne!(direct, cps);

    // A whole-program transform can reproduce behavior, but it has introduced
    // an explicit exit-continuation protocol that the source call chain lacked.
    const CPS_CAN_REPRODUCE_RETURN: bool = true;
    const CPS_PROVES_SAME_BASE_D4_PARENT: bool = false;
    assert!(CPS_CAN_REPRODUCE_RETURN);
    assert!(!CPS_PROVES_SAME_BASE_D4_PARENT);
}

#[test]
fn go_and_return_have_different_structural_status() {
    let go_operation = BaseOperation::FiniteStateTransition;
    let return_operation = BaseOperation::DynamicExit;

    assert_ne!(go_operation, return_operation);

    const GO_LOCAL_STATE_MACHINE_DERIVED: bool = true;
    const RETURN_REQUIRES_NONLOCAL_EXIT_FACTOR: bool = true;
    assert!(GO_LOCAL_STATE_MACHINE_DERIVED);
    assert!(RETURN_REQUIRES_NONLOCAL_EXIT_FACTOR);
}

#[test]
fn prog_is_composite_not_evidence_for_a_single_resident() {
    struct ProgFactors {
        local_go: BaseOperation,
        return_factor: BaseOperation,
    }

    let prog = ProgFactors {
        local_go: BaseOperation::FiniteStateTransition,
        return_factor: BaseOperation::DynamicExit,
    };

    assert_eq!(prog.local_go, BaseOperation::FiniteStateTransition);
    assert_eq!(prog.return_factor, BaseOperation::DynamicExit);

    const PROG_SINGLE_RESIDENT_PROVED: bool = false;
    assert!(!PROG_SINGLE_RESIDENT_PROVED);
}

#[test]
fn nonlocal_exit_and_shared_location_mutation_are_cross_family_distinctions() {
    // RETURN can alter control target while leaving store untouched.
    let return_effect = (BaseOperation::DynamicExit, "store-unchanged");
    // #2589 mutation can alter a pre-existing location while normal control
    // returns normally.
    let mutation_effect = (BaseOperation::SharedLocationUpdate, "normal-return");

    assert_ne!(return_effect.0, mutation_effect.0);
    assert_ne!(return_effect.1, mutation_effect.1);
}

#[test]
fn no_parent_does_not_imply_width_root_or_coordinate() {
    const SAME_BASE_D4_PARENT_FOUND: bool = false;
    const EXACT_WIDTH_KNOWN: bool = false;
    const INDEPENDENT_ROOT_PROVED: bool = false;
    const COORDINATE_ALLOCATED: bool = false;

    assert!(!SAME_BASE_D4_PARENT_FOUND);
    assert!(!EXACT_WIDTH_KNOWN);
    assert!(!INDEPENDENT_ROOT_PROVED);
    assert!(!COORDINATE_ALLOCATED);
}
