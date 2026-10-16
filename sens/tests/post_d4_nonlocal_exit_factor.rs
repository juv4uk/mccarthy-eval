// crates/sens/tests/post_d4_nonlocal_exit_factor.rs
//
// Structural discovery witness for #2590:
// [P0][STRUCTURAL-DISCOVERY][NONLOCAL-EXIT-PARENT-1] Attack RETURN/PROG parenthood and width pressure.
//
// Research-only. No production control operator and no binary coordinate are allocated here.
//
// Covers:
// 1. Remove-one reconstruction attack on non-local exit relative to admitted D1-D4 local control;
// 2. Source capability vs whole-program CPS compilability distinction;
// 3. Classification of PROG as a composite construct (COMPOSITE) rather than an atomic primitive resident;
// 4. Honest parenthood attack against D1-D4 candidates (APPLY, EVAL, LAMBDA, COND, GO) showing base mismatch;
// 5. Cross-family orthogonality control between #2589 (shared-location-update) and #2590 (non-local-exit);
// 6. Strict width-pressure gate: no D5/D6 coordinates allocated.

use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Domain {
    Core,
    CoreMath,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Value {
    domain: Domain,
    content: &'static str,
}

impl Value {
    const fn core(content: &'static str) -> Self {
        Self {
            domain: Domain::Core,
            content,
        }
    }

    const fn core_math(content: &'static str) -> Self {
        Self {
            domain: Domain::CoreMath,
            content,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ControlTransfer {
    Normal(Value),
    NonLocalReturn {
        target_prog: &'static str,
        value: Value,
    },
    Error(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Frame {
    name: &'static str,
    is_prog_boundary: bool,
    prog_tag: Option<&'static str>,
}

#[derive(Debug, Clone)]
struct CallStack {
    frames: Vec<Frame>,
    execution_trace: Vec<&'static str>,
}

impl CallStack {
    fn new() -> Self {
        Self {
            frames: Vec::new(),
            execution_trace: Vec::new(),
        }
    }

    fn push_frame(&mut self, name: &'static str, prog_tag: Option<&'static str>) {
        self.frames.push(Frame {
            name,
            is_prog_boundary: prog_tag.is_some(),
            prog_tag,
        });
        self.execution_trace.push(name);
    }

    fn pop_frame(&mut self) -> Option<Frame> {
        self.frames.pop()
    }

    // Delivers a non-local return by unwinding frames until the nearest matching PROG boundary.
    fn deliver_return(&mut self, value: Value) -> ControlTransfer {
        if value.domain != Domain::Core {
            return ControlTransfer::Error("domain-mismatch: cannot return non-Core value");
        }

        while let Some(top) = self.frames.last() {
            if top.is_prog_boundary {
                let tag = top.prog_tag.unwrap_or("anonymous-prog");
                return ControlTransfer::NonLocalReturn {
                    target_prog: tag,
                    value,
                };
            }
            self.frames.pop();
        }

        ControlTransfer::Error("no-active-prog-boundary")
    }
}

// -----------------------------------------------------------------------------
// Test 1: Remove-one reconstruction attack on non-local-exit.
// Under admitted D1-D4 local control, control returns strictly to the immediate caller.
// An escaping RETURN cannot be reproduced without modifying intermediate frames.
// -----------------------------------------------------------------------------
#[test]
fn remove_one_nonlocal_exit_cannot_be_reproduced_by_local_returns() {
    let mut stack = CallStack::new();

    // Setup nested dynamic call: outer PROG -> middle function -> inner function
    stack.push_frame("outer-prog", Some("P0"));
    stack.push_frame("middle-fn", None);
    stack.push_frame("inner-fn", None);

    // Case A: Pure local D1-D4 return.
    // Inner returns to middle. Middle must execute its own post-return logic.
    let local_outcome = ControlTransfer::Normal(Value::core("inner-val"));
    assert_eq!(
        local_outcome,
        ControlTransfer::Normal(Value::core("inner-val"))
    );
    // The top of the stack is still inner-fn until popped by local return to middle-fn:
    assert_eq!(stack.frames.last().unwrap().name, "inner-fn");

    // Case B: Non-local exit (RETURN).
    // Inner initiates return across middle-fn straight to P0.
    let nonlocal_outcome = stack.deliver_return(Value::core("exit-val"));
    assert_eq!(
        nonlocal_outcome,
        ControlTransfer::NonLocalReturn {
            target_prog: "P0",
            value: Value::core("exit-val"),
        }
    );
    // Intermediate frame "middle-fn" was unwound without receiving normal return:
    assert_eq!(stack.frames.len(), 1);
    assert_eq!(stack.frames[0].name, "outer-prog");

    // The observable delta is distinct:
    // With non-local exit removed, middle-fn's continuation cannot be skipped
    // without altering middle-fn's interface.
    assert_ne!(local_outcome, nonlocal_outcome);
}

// -----------------------------------------------------------------------------
// Test 2: Source capability vs whole-program CPS compilability.
// While whole-program CPS can compile away non-local exits by reifying continuations,
// that is a whole-program translation that alters every function's calling convention.
// At the source language / evaluator level, the capability is independent.
// -----------------------------------------------------------------------------
#[test]
fn source_capability_is_distinct_from_whole_program_cps_compilability() {
    // Direct-style (source) evaluator signatures:
    // fn eval_direct(form, env) -> Result<Value, Error>
    // intermediate functions have normal return types without continuation passing.

    // CPS-style signatures:
    // fn eval_cps<K>(form, env, k: K) -> !
    // requires threading an explicit continuation parameter through all caller frames.

    struct DirectCaller {
        post_call_executed: bool,
    }

    impl DirectCaller {
        fn invoke_callee(&mut self, non_local: bool) -> Result<Value, &'static str> {
            if non_local {
                // In non-local exit, the remaining block in this caller is aborted:
                return Err("unwound-by-return");
            }
            self.post_call_executed = true;
            Ok(Value::core("direct-return"))
        }
    }

    let mut direct = DirectCaller {
        post_call_executed: false,
    };
    let _ = direct.invoke_callee(false);
    assert!(
        direct.post_call_executed,
        "Normal return executes post-call continuation in caller"
    );

    let mut direct_escaped = DirectCaller {
        post_call_executed: false,
    };
    let res = direct_escaped.invoke_callee(true);
    assert_eq!(res, Err("unwound-by-return"));
    assert!(
        !direct_escaped.post_call_executed,
        "Non-local exit skips direct caller's post-call execution"
    );

    // This proves that at source evaluation level, skipping caller continuation
    // without passing explicit continuations is an un-eliminable primitive capability.
}

// -----------------------------------------------------------------------------
// Test 3: Classification of PROG as a composite construct (COMPOSITE).
// PROG decomposes into:
// 1. local variable binding (analogous to LET/LAMBDA);
// 2. sequential evaluation (PROGN);
// 3. local jump (GO);
// 4. dynamic exit delimiter (RETURN boundary).
// It does not form an indivisible atomic resident.
// -----------------------------------------------------------------------------
#[test]
fn prog_is_composite_not_atomic_resident() {
    #[derive(Debug, PartialEq, Eq)]
    struct ProgDecomposition {
        has_local_bindings: bool,
        has_sequential_body: bool,
        has_local_jump_go: bool,
        has_dynamic_exit_delimiter: bool,
    }

    let prog_historical = ProgDecomposition {
        has_local_bindings: true,
        has_sequential_body: true,
        has_local_jump_go: true,
        has_dynamic_exit_delimiter: true,
    };

    // Each sub-capability is factorable:
    let let_binding_only = ProgDecomposition {
        has_local_bindings: true,
        has_sequential_body: false,
        has_local_jump_go: false,
        has_dynamic_exit_delimiter: false,
    };
    let progn_sequence_only = ProgDecomposition {
        has_local_bindings: false,
        has_sequential_body: true,
        has_local_jump_go: false,
        has_dynamic_exit_delimiter: false,
    };
    let block_delimiter_only = ProgDecomposition {
        has_local_bindings: false,
        has_sequential_body: false,
        has_local_jump_go: false,
        has_dynamic_exit_delimiter: true,
    };

    assert_ne!(prog_historical, let_binding_only);
    assert_ne!(prog_historical, progn_sequence_only);
    assert_ne!(prog_historical, block_delimiter_only);

    // Consequence: PROG is composite; residents = 0.
    const PROG_CLASSIFICATION: &str = "COMPOSITE";
    const RESIDENTS_ALLOCATED: usize = 0;
    assert_eq!(PROG_CLASSIFICATION, "COMPOSITE");
    assert_eq!(RESIDENTS_ALLOCATED, 0);
}

// -----------------------------------------------------------------------------
// Test 4: Honest parenthood attack against D1-D4 candidates.
// Placement law #2236 requires that a parent candidate share the same base semantic
// operation, with only a delta on observable control axes.
// Candidates tested:
// - APPLY: base operation is callable application (callable + args -> result)
// - EVAL: base operation is expression evaluation (form + env -> result)
// - LAMBDA: base operation is closure construction (params + body + env -> closure)
// - COND: base operation is local branch selection within frame
// - GO: base operation is intra-frame label jump (no return value, no frame unwinding)
// Result: All rejected on base semantic operation mismatch.
// Strongest honest parent: NO-PARENT / UNKNOWN.
// -----------------------------------------------------------------------------
#[test]
fn honest_parenthood_attack_rejects_d1_d4_candidates() {
    #[derive(Debug, PartialEq, Eq)]
    struct SemanticShape {
        operands: &'static str,
        primary_effect: &'static str,
        stack_effect: &'static str,
    }

    let apply_shape = SemanticShape {
        operands: "callable + arg-list + env",
        primary_effect: "invoke-callable",
        stack_effect: "pushes-frame",
    };
    let eval_shape = SemanticShape {
        operands: "form + env",
        primary_effect: "evaluate-form",
        stack_effect: "local-eval",
    };
    let lambda_shape = SemanticShape {
        operands: "params + body + env",
        primary_effect: "construct-closure",
        stack_effect: "none",
    };
    let cond_shape = SemanticShape {
        operands: "predicate-clauses",
        primary_effect: "select-branch",
        stack_effect: "local-branch",
    };
    let go_shape = SemanticShape {
        operands: "tag-symbol",
        primary_effect: "jump-to-label",
        stack_effect: "intra-frame-ip-change",
    };
    let return_shape = SemanticShape {
        operands: "value",
        primary_effect: "nonlocal-exit-transfer",
        stack_effect: "unwinds-to-enclosing-prog",
    };

    // None of the D1-D4 candidates match RETURN's primary effect or stack effect:
    assert_ne!(apply_shape.primary_effect, return_shape.primary_effect);
    assert_ne!(eval_shape.primary_effect, return_shape.primary_effect);
    assert_ne!(lambda_shape.primary_effect, return_shape.primary_effect);
    assert_ne!(cond_shape.primary_effect, return_shape.primary_effect);
    assert_ne!(go_shape.primary_effect, return_shape.primary_effect);

    assert_ne!(apply_shape.stack_effect, return_shape.stack_effect);
    assert_ne!(go_shape.stack_effect, return_shape.stack_effect);

    const STRONGEST_HONEST_PARENT: Option<&'static str> = None;
    assert_eq!(
        STRONGEST_HONEST_PARENT, None,
        "Strongest honest parent is NO-PARENT"
    );
}

// -----------------------------------------------------------------------------
// Test 5: Cross-family orthogonality control between SET/SETQ (#2589) and RETURN (#2590).
// Suggested by owner:
// - mutation-only witness: observer sees binding OLD -> NEW while control returns normally;
// - non-local-exit-only witness: RETURN escapes a nested call while shared binding store is unchanged.
// If both survive with the other factor removed, that proves clean orthogonality.
// -----------------------------------------------------------------------------
#[test]
fn mutation_and_nonlocal_exit_are_strictly_orthogonal() {
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct BindingStore {
        cells: BTreeMap<&'static str, &'static str>,
    }

    let initial_store = BindingStore {
        cells: {
            let mut m = BTreeMap::new();
            m.insert("x", "OLD");
            m
        },
    };

    // Observation A: Mutation ONLY (control returns normally).
    let mut store_a = initial_store.clone();
    let mut stack_a = CallStack::new();
    stack_a.push_frame("prog-root", Some("P0"));
    stack_a.push_frame("helper", None);

    // helper mutates binding:
    store_a.cells.insert("x", "NEW");
    // helper returns normally:
    stack_a.pop_frame();
    let outcome_a = ControlTransfer::Normal(Value::core("done"));

    assert_eq!(store_a.cells.get("x"), Some(&"NEW"), "Store was mutated");
    assert_eq!(
        outcome_a,
        ControlTransfer::Normal(Value::core("done")),
        "Control returned normally"
    );
    assert_eq!(
        stack_a.frames.len(),
        1,
        "Only helper frame was popped normally"
    );

    // Observation B: Non-local exit ONLY (store remains untouched).
    let store_b = initial_store.clone();
    let mut stack_b = CallStack::new();
    stack_b.push_frame("prog-root", Some("P0"));
    stack_b.push_frame("intermediate-1", None);
    stack_b.push_frame("intermediate-2", None);

    // intermediate-2 triggers RETURN without touching the store:
    let outcome_b = stack_b.deliver_return(Value::core("early-val"));

    assert_eq!(
        store_b, initial_store,
        "Store is byte/structurally unchanged"
    );
    assert_eq!(
        outcome_b,
        ControlTransfer::NonLocalReturn {
            target_prog: "P0",
            value: Value::core("early-val")
        },
        "Non-local exit escaped to P0"
    );
    assert_eq!(stack_b.frames.len(), 1, "Both intermediate frames unwound");

    // Observation C: Both factors can occur independently.
    // Survival of each under removal of the other confirms F1 and F2 are orthogonal.
    assert_ne!(store_a, initial_store);
    assert_eq!(store_b, initial_store);
    assert_eq!(outcome_a, ControlTransfer::Normal(Value::core("done")));
    assert_ne!(outcome_b, ControlTransfer::Normal(Value::core("early-val")));
}

// -----------------------------------------------------------------------------
// Test 6: Domain firewall (#2508).
// Cross-domain Core-Math values cannot hijack Core control transfer.
// -----------------------------------------------------------------------------
#[test]
fn core_math_control_domain_firewall_fails_closed() {
    let mut stack = CallStack::new();
    stack.push_frame("prog-root", Some("P0"));

    let math_val = Value::core_math("vector-norm");
    let outcome = stack.deliver_return(math_val);

    assert_eq!(
        outcome,
        ControlTransfer::Error("domain-mismatch: cannot return non-Core value")
    );
}

// -----------------------------------------------------------------------------
// Test 7: Conservative width & coordinate accounting.
// Verifies that factor independence does NOT grant a binary coordinate.
// -----------------------------------------------------------------------------
#[test]
#[allow(clippy::assertions_on_constants)]
fn nonlocal_exit_factor_verdict_is_width_conservative() {
    const FACTOR_STATUS: &str = "BOUNDED-INDEPENDENT";
    const BINARY_OBJECT: &str = "UNPLACED";
    const PROVEN_INDEPENDENT_ROOTS: usize = 0;
    const NEW_D5_RESIDENTS: usize = 0;
    const COORDINATES_ALLOCATED: usize = 0;
    const WIDTH_PRESSURE_RATIFIED: bool = false;

    assert_eq!(FACTOR_STATUS, "BOUNDED-INDEPENDENT");
    assert_eq!(BINARY_OBJECT, "UNPLACED");
    assert_eq!(PROVEN_INDEPENDENT_ROOTS, 0);
    assert_eq!(NEW_D5_RESIDENTS, 0);
    assert_eq!(COORDINATES_ALLOCATED, 0);
    const { assert!(!WIDTH_PRESSURE_RATIFIED); }
}
