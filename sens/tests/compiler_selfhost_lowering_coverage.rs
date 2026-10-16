//! sens#3823 — whole-nucleus coverage over the production SENS-owned role API.
//!
//! This test owns no identity->role semantics. It lowers the real current
//! compiler nucleus, observes exact DomainCall identities, and asks
//! `compiler_lowering_role_from_sens` for the already-derived role.

use sens::syntax::{Expr, ExprKind};
use sens::{
    compiler_lowering_role_from_sens, lower_program, parse_mixed_exact_domain, Bit4, Bit5, Bit8,
    CompilerLoweringRole, CoreD4, CoreD5, CoreD8, CoreDomainIdentity,
};

const NUCLEUS: &str = include_str!("../../../lib/compiler-nucleus.lisp");
const PRODUCTION_ADAPTER: &str = include_str!("../src/compiler_language.rs");

fn assert_cond_clause_shapes(expr: &Expr) {
    match &expr.kind {
        ExprKind::DomainCall(identity, args) => {
            let role = compiler_lowering_role_from_sens(*identity)
                .expect("current compiler identity must have a decidable lowering role")
                .expect("current compiler DomainCall must have an admitted lowering role");

            if role == CompilerLoweringRole::CondForm {
                assert!(
                    !args.is_empty(),
                    "current compiler nucleus contains an empty exact COND"
                );
                for (index, clause) in args.iter().enumerate() {
                    let ExprKind::List(parts) = &clause.kind else {
                        panic!(
                            "current compiler exact COND clause {index} is not a source list: {:?}",
                            clause.kind
                        );
                    };
                    assert_eq!(
                        parts.len(),
                        2,
                        "current compiler exact COND clause {index} must be exactly (test expression)"
                    );
                }
            }

            for arg in args.iter() {
                assert_cond_clause_shapes(arg);
            }
        }
        ExprKind::List(items) => {
            for item in items.iter() {
                assert_cond_clause_shapes(item);
            }
        }
        ExprKind::Pair(head, tail) => {
            assert_cond_clause_shapes(head);
            assert_cond_clause_shapes(tail);
        }
        ExprKind::Sid(_)
        | ExprKind::Call(_, _)
        | ExprKind::Number(_, _)
        | ExprKind::Rational(_)
        | ExprKind::BinaryNumber(_)
        | ExprKind::NumericBuffer(_)
        | ExprKind::DomainIdentity(_)
        | ExprKind::String(_)
        | ExprKind::Symbol(_)
        | ExprKind::Local { .. } => {}
    }
}

fn collect_roles(expr: &Expr, roles: &mut Vec<CompilerLoweringRole>) {
    match &expr.kind {
        ExprKind::DomainCall(identity, args) => {
            let role = compiler_lowering_role_from_sens(*identity)
                .unwrap_or_else(|error| {
                    panic!(
                        "SENS-owned role derivation failed for current nucleus identity {identity:?}: {error}"
                    )
                })
                .unwrap_or_else(|| {
                    panic!(
                        "current compiler nucleus lowered an exact domain call without an admitted lowering role: {identity:?}"
                    )
                });

            if !roles.contains(&role) {
                roles.push(role);
            }

            for arg in args.iter() {
                collect_roles(arg, roles);
            }
        }
        ExprKind::List(items) => {
            for item in items.iter() {
                collect_roles(item, roles);
            }
        }
        ExprKind::Pair(head, tail) => {
            collect_roles(head, roles);
            collect_roles(tail, roles);
        }
        ExprKind::Sid(_)
        | ExprKind::Call(_, _)
        | ExprKind::Number(_, _)
        | ExprKind::Rational(_)
        | ExprKind::BinaryNumber(_)
        | ExprKind::NumericBuffer(_)
        | ExprKind::DomainIdentity(_)
        | ExprKind::String(_)
        | ExprKind::Symbol(_)
        | ExprKind::Local { .. } => {}
    }
}

#[test]
fn every_exact_domain_call_in_current_compiler_nucleus_has_a_sens_owned_role() {
    let parsed = parse_mixed_exact_domain(NUCLEUS).expect("current compiler nucleus source must parse");
    let lowered = lower_program(&parsed);

    let mut roles = Vec::new();
    for expr in &lowered {
        collect_roles(expr, &mut roles);
    }

    for expected in [
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
            roles.contains(&expected),
            "current compiler nucleus did not exercise expected SENS-owned lowering role {expected:?}; observed {roles:?}"
        );
    }
}

#[test]
fn every_exact_cond_in_current_compiler_nucleus_has_two_part_clauses() {
    let parsed = parse_mixed_exact_domain(NUCLEUS).expect("current compiler nucleus source must parse");
    let lowered = lower_program(&parsed);

    for expr in &lowered {
        assert_cond_clause_shapes(expr);
    }
}

#[test]
fn equal_payloads_outside_the_owning_domain_stay_fail_closed() {
    let collisions = [
        CoreDomainIdentity::D4(CoreD4::from_word(Bit4::new(0b0001).unwrap())),
        CoreDomainIdentity::D5(CoreD5::from_word(Bit5::new(0b00111).unwrap())),
        CoreDomainIdentity::D8(CoreD8::from_word(Bit8::new(0b0000_0010).unwrap())),
    ];

    for identity in collisions {
        assert_eq!(
            compiler_lowering_role_from_sens(identity).expect("role query"),
            None,
            "equal payload outside the owning domain must not inherit compiler meaning: {identity:?}"
        );
    }
}

#[test]
fn production_role_path_contains_no_host_owned_identity_to_role_projection() {
    for forbidden in [
        "d3_syntax_kind",
        "domain_primitive_kind",
        "identity_for_domain_identity",
        "match word.word().packed_bits()",
    ] {
        assert!(
            !PRODUCTION_ADAPTER.contains(forbidden),
            "production adapter must execute the SENS-owned role law, not project roles through {forbidden}"
        );
        assert!(
            !NUCLEUS.contains(forbidden),
            "SENS compiler nucleus must remain structural and backend-neutral: found {forbidden}"
        );
    }
}
