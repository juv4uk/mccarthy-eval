//! sens#3824 — whole-current-nucleus role closure through executable SENS law.
//!
//! The inventory comes from parsing/lowering the real compiler nucleus.  This
//! test contains no DomainIdentity->role table.

use sens::syntax::{Expr, ExprKind};
use sens::{
    compiler_execution_role_from_sens, compiler_lowering_role_from_sens, lower_program, parse_mixed_exact_domain,
    Bit4, Bit5, Bit8, CompilerExecutionRole, CompilerLoweringRole, CoreD4, CoreD5, CoreD8,
    CoreDomainIdentity,
};

const NUCLEUS: &str = include_str!("../../../lib/compiler-nucleus.lisp");

fn collect_roles(expr: &Expr, roles: &mut Vec<CompilerLoweringRole>) {
    match &expr.kind {
        ExprKind::DomainCall(identity, args) => {
            let role = compiler_lowering_role_from_sens(*identity)
                .unwrap_or_else(|error| panic!("SENS lowering-role execution failed: {error:?}"))
                .unwrap_or_else(|| {
                    panic!(
                        "current compiler nucleus contains an exact-domain call without a selfhost lowering role: {identity}"
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
fn real_compiler_nucleus_is_closed_over_nine_sens_derived_roles() {
    let parsed = parse_mixed_exact_domain(NUCLEUS).expect("current compiler nucleus parses");
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
            "current compiler nucleus did not exercise expected SENS-derived role {expected:?}; observed {roles:?}"
        );
    }

    assert_eq!(roles.len(), 9, "bounded current nucleus closure must stay explicit");
}

#[test]
fn equal_payloads_in_other_domains_do_not_gain_selfhost_roles() {
    let cases = [
        CoreDomainIdentity::D4(CoreD4::from_word(Bit4::new(0b0001).unwrap())),
        CoreDomainIdentity::D5(CoreD5::from_word(Bit5::new(0b00010).unwrap())),
        CoreDomainIdentity::D8(CoreD8::from_word(Bit8::new(0b0000_0010).unwrap())),
    ];

    for identity in cases {
        assert_eq!(
            compiler_lowering_role_from_sens(identity)
                .expect("wrong-domain query is a normal fail-closed result"),
            None,
            "equal packed payload under another domain must not inherit a compiler role"
        );
    }
}

#[test]
fn established_three_role_execution_api_remains_compatibility_stable() {
    let parsed = parse_mixed_exact_domain(NUCLEUS).expect("current compiler nucleus parses");
    let lowered = lower_program(&parsed);

    let mut saw_old_role = false;
    fn walk(expr: &Expr, saw_old_role: &mut bool) {
        match &expr.kind {
            ExprKind::DomainCall(identity, args) => {
                if let Some(role) = compiler_execution_role_from_sens(*identity)
                    .expect("bounded execution-role API remains executable")
                {
                    assert!(matches!(
                        role,
                        CompilerExecutionRole::SelectorHead
                            | CompilerExecutionRole::SelectorTail
                            | CompilerExecutionRole::PairConstruct
                    ));
                    *saw_old_role = true;
                }
                for arg in args.iter() {
                    walk(arg, saw_old_role);
                }
            }
            ExprKind::List(items) => {
                for item in items.iter() {
                    walk(item, saw_old_role);
                }
            }
            ExprKind::Pair(head, tail) => {
                walk(head, saw_old_role);
                walk(tail, saw_old_role);
            }
            _ => {}
        }
    }

    for expr in &lowered {
        walk(expr, &mut saw_old_role);
    }
    assert!(saw_old_role, "current nucleus must still exercise the merged three-role vertical");
}
