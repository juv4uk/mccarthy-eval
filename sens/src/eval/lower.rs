//! Одноразове зведення голови виклику до канонічної доменної identity.
//!
//! Surface-и, для яких уже ратифіковано exact domain identity, стають
//! `ExprKind::DomainCall`. Старий точний 8-бітний token лишається окремим
//! compatibility-шляхом `ExprKind::Call` зі старим байтом: lower ніколи не
//! виводить домен із історичного байта.
//!
//! M8 (#1590): зводяться **усі** admitted surface з реєстру, не лише Canon
//! і necessary forms. Написання (`+`, `-`, `додати`, …) — маршрутизація до
//! фіксованого SENS, не окрема identity, яку можна перевизначити й тим
//! самим змусити runtime шукати ім'я на кожному виклику (#1413).
//!
//! Дані лишаються даними: аргумент `quote`, клаузи `cond` (самі клаузи — не
//! виклики), параметри `lambda` та ім'я в `def`/`define`.

use super::{canon, necessary_forms};
use crate::semantic_registry;
use crate::syntax::{Expr, ExprKind, MAX_STRUCTURE_DEPTH};
use crate::CoreDomainIdentity;
use crate::CoreDomainIdentity;
use std::rc::Rc;

#[derive(Clone, Copy, Eq, PartialEq)]
enum LowerMode {
    Historical,
    ExactDomain,
}

/// Legacy source only: W8 compatibility does not confer canonical D8 identity.
pub fn lower_program(expressions: &[Expr]) -> Vec<Expr> {
    expressions
        .iter()
        .map(|expression| lower(expression, 0, LowerMode::Historical))
        .collect()
}

/// Exact D1–D9 source: no W8 fallback from bytes or historical surfaces.
pub(crate) fn lower_exact_domain_program(expressions: &[Expr]) -> Vec<Expr> {
    expressions
        .iter()
        .map(|expression| lower(expression, 0, LowerMode::ExactDomain))
        .collect()
}

fn head_domain_identity(head: &Expr, mode: LowerMode) -> Option<CoreDomainIdentity> {
    match &head.kind {
        ExprKind::DomainIdentity(identity) => identity.core_operation(),
        ExprKind::Symbol(name) if mode == LowerMode::ExactDomain => {
            semantic_registry::domain_identity_for_surface(name)
        }
        ExprKind::Symbol(name) => immutable_surface_domain_identity(name),
        _ => None,
    }
}

/// The source-surface → exact-domain projection boundary, shared by lowering
/// and immutable-binding checks. Never reconstruct an historical eight-bit ID.
pub(crate) fn immutable_surface_domain_identity(name: &str) -> Option<CoreDomainIdentity> {
    necessary_forms::domain_identity_for_symbol(name)
        .or_else(|| semantic_registry::domain_identity_for_surface(name))
}

fn head_sid(head: &Expr) -> Option<CoreDomainIdentity> {
    match &head.kind {
        ExprKind::Sid(sid) => Some(*sid),
        ExprKind::Symbol(name) => immutable_surface_sid(name),
        _ => None,
    }
}

/// Написання, що маршрутизується до фіксованого SENS (не окрема identity).
fn immutable_surface_sid(name: &str) -> Option<CoreDomainIdentity> {
    // Once a surface has an exact domain identity, the legacy byte route is no
    // longer allowed to win. Unmigrated registry rows keep the old path.
    if immutable_surface_domain_identity(name).is_some() {
        return None;
    }
    if let Some(sid) = canon::routed_sid_for_surface(name) {
        return Some(sid);
    }
    semantic_registry::admitted_semantic_id_for_surface(name)
}

fn is_d3(identity: CoreDomainIdentity, bits: u8) -> bool {
    matches!(
        identity,
        CoreDomainIdentity::D3(word) if word.word().packed_bits() == bits
    )
}

/// Сумісна голова W8 одержує закон лише через доказову D3-проєкцію.
fn historical_head_has_d3_law(sid: CoreDomainIdentity, bits: u8) -> bool {
    semantic_registry::compatibility_d3_route_from_ratified_domains(sid)
        .is_some_and(|identity| is_d3(identity, bits))
}

fn lower_all(items: &[Expr], depth: u32, mode: LowerMode) -> Rc<[Expr]> {
    items
        .iter()
        .map(|item| lower(item, depth + 1, mode))
        .collect()
}

/// Lower an already typed or newly discovered domain call in exactly the
/// same way. A quoted datum and a D4 binder are not executable call heads.
fn lower_domain_arguments(
    identity: CoreDomainIdentity,
    arguments: &[Expr],
    depth: u32,
    mode: LowerMode,
) -> Rc<[Expr]> {
    if is_d3(identity, 0b001) {
        return arguments.into();
    }
    if is_d3(identity, 0b110) {
        return arguments
            .iter()
            .map(|clause| match &clause.kind {
                ExprKind::List(parts) => Expr {
                    kind: ExprKind::List(lower_all(parts, depth + 1, mode)),
                    span: clause.span,
                },
                _ => clause.clone(),
            })
            .collect();
    }
    if necessary_forms::identity_for_domain_identity(identity).is_some() {
        return arguments
            .iter()
            .enumerate()
            .map(|(index, argument)| {
                if index == 0 {
                    argument.clone()
                } else {
                    lower(argument, depth + 1, mode)
                }
            })
            .collect();
    }
    lower_all(arguments, depth, mode)
}

/// Preserve historical QUOTE/COND/binder argument shape while continuing
/// staged lowering inside an already-typed legacy call. This is a compatibility
/// mechanism only: no W8 word is promoted to current D8 or admitted by payload.
fn lower_historical_arguments(
    sid: CoreDomainIdentity,
    arguments: &[Expr],
    depth: u32,
    mode: LowerMode,
) -> Rc<[Expr]> {
    if historical_head_has_d3_law(sid, 0b001) {
        return arguments.into();
    }
    if historical_head_has_d3_law(sid, 0b110) {
        return arguments
            .iter()
            .map(|clause| match &clause.kind {
                ExprKind::List(parts) => Expr {
                    kind: ExprKind::List(lower_all(parts, depth + 1, mode)),
                    span: clause.span,
                },
                _ => clause.clone(),
            })
            .collect();
    }
    if necessary_forms::identity_for_semantic_id(sid).is_some() {
        return arguments
            .iter()
            .enumerate()
            .map(|(index, argument)| {
                if index == 0 {
                    argument.clone()
                } else {
                    lower(argument, depth + 1, mode)
                }
            })
            .collect();
    }
    lower_all(arguments, depth, mode)
}

fn lower(expression: &Expr, depth: u32, mode: LowerMode) -> Expr {
    if depth > MAX_STRUCTURE_DEPTH {
        return expression.clone();
    }
    let kind = match &expression.kind {
        ExprKind::Call(sid, arguments) if mode == LowerMode::Historical => {
            ExprKind::Call(*sid, lower_historical_arguments(*sid, arguments, depth, mode))
        }
        // Exact-domain lowering never interprets a pre-existing historical
        // Call as an admitted current-domain operation.
        ExprKind::Call(sid, arguments) => ExprKind::Call(*sid, arguments.clone()),
        ExprKind::DomainCall(identity, arguments) => {
            ExprKind::DomainCall(*identity, lower_domain_arguments(*identity, arguments, depth, mode))
        }
        ExprKind::List(items) if !items.is_empty() => {
            let arguments = &items[1..];

            if let Some(identity) = head_domain_identity(&items[0], mode) {
                return Expr {
                    kind: ExprKind::DomainCall(
                        identity,
                        lower_domain_arguments(identity, arguments, depth, mode),
                    ),
                    span: expression.span,
                };
            }

            // Exact source must not consult a flat historical W8 registry.
            if mode == LowerMode::ExactDomain {
                return Expr {
                    kind: ExprKind::List(lower_all(items, depth, mode)),
                    span: expression.span,
                };
            }

            match head_sid(&items[0]) {
                Some(sid) => ExprKind::Call(
                    sid,
                    lower_historical_arguments(sid, arguments, depth, mode),
                ),
                None => ExprKind::List(lower_all(items, depth, mode)),
            }
        }
        _ => return expression.clone(),
    };
    Expr {
        kind,
        span: expression.span,
    }
}

#[cfg(test)]
mod exact_domain_lowering_tests {
    use super::*;

    /// A partially lowered AST is a valid compiler input. Typed call heads
    /// must not shield their nested domain calls from lowering.
    fn compare_partial_and_source(source: &str) {
        let parsed = crate::parser::parse_exact_domain_source(source)
            .expect("valid current exact-domain source");
        let ExprKind::List(items) = &parsed[0].kind else {
            panic!("top-level call required");
        };
        let ExprKind::DomainIdentity(domain) = items[0].kind else {
            panic!("current domain head required");
        };
        let identity = domain.core_operation().expect("admitted callable domain");
        let partial = Expr {
            kind: ExprKind::DomainCall(identity, items[1..].into()),
            span: parsed[0].span,
        };
        let canonical = lower_program(&parsed);
        let lowered = lower_program(&[partial]);
        assert_eq!(lowered, canonical, "typed and source heads must lower alike");
        assert_eq!(lower_program(&lowered), lowered, "lowering must be idempotent");
    }

    #[test]
    fn nested_d3_calls_are_lowered_under_an_already_typed_head() {
        compare_partial_and_source("(111 (100 (001 000)) (011 (001 000)))");
    }

    #[test]
    fn quote_data_is_not_rewritten_as_executable_domain_calls() {
        compare_partial_and_source("(001 (010 (001 11110111)))");
    }

    #[test]
    fn lambda_binder_stays_data_while_its_body_is_lowered() {
        compare_partial_and_source("(0010 (x) (010 (001 x)))");
    }

    #[test]
    fn cond_clause_structure_and_nested_calls_survive_lowering() {
        compare_partial_and_source("(110 ((001 1) (010 (001 000))))");
    }
}


#[cfg(test)]
mod domain_ladder_lowering_migration_tests {
    use super::*;

    fn lowered_one(source: &str) -> Expr {
        let parsed = crate::parse(source).expect("сумісний читач");
        lower_program(&parsed).into_iter().next().expect("одна форма")
    }

    #[test]
    fn staged_historical_call_lowers_its_body_without_rewriting_quote_or_binders() {
        // Mechanical AST projection only: language semantics are proved by
        // the native binary SENS witnesses, not by this host-language test.
        for source in [
            "(00000001 (00000101 x))",
            "(00000111 ((00000101 x) (00000001 (00000101 y))))",
            "(00001000 (x) (00000001 (00000101 x)))",
            "(00001011 x (00000101 y))",
            "(00000110 (00000001 x) (00000101 y))",
        ] {
            let parsed = crate::parse(source).expect("valid historical source");
            let ExprKind::List(items) = &parsed[0].kind else {
                panic!("historical source must have a list head");
            };
            let ExprKind::Sid(sid) = &items[0].kind else {
                panic!("historical list head must be an exact W8 token");
            };
            let staged = Expr {
                kind: ExprKind::Call(*sid, items[1..].into()),
                span: parsed[0].span,
            };
            let expected = lower_program(&parsed);
            let actual = lower_program(&[staged]);
            assert_eq!(actual, expected, "staged vs source lowering: {source}");
            assert_eq!(lower_program(&actual), actual, "idempotence: {source}");
        }
    }

    #[test]
    fn historical_quote_does_not_evaluate_quoted_data() {
        let lowered = lowered_one("(00000001 (00000101 x))");
        let ExprKind::Call(_, args) = lowered.kind else { panic!("QUOTE має лишатися W8 для сумісного читача"); };
        assert_eq!(args.len(), 1);
        assert!(matches!(&args[0].kind, ExprKind::List(items) if matches!(&items[0].kind, ExprKind::Sid(_))));
    }

    #[test]
    fn historical_cond_lowers_only_clause_expressions() {
        let lowered = lowered_one("(00000111 ((00000101 x) (00000001 (00000101 y))))");
        let ExprKind::Call(_, clauses) = lowered.kind else { panic!("COND має залишатися W8 для сумісності"); };
        let ExprKind::List(parts) = &clauses[0].kind else { panic!("структурна клауза"); };
        assert!(matches!(&parts[0].kind, ExprKind::Call(_, _)));
        assert!(matches!(&parts[1].kind, ExprKind::Call(_, _)));
    }

    #[test]
    fn unrelated_w8_cannot_infer_d3_law() {
        for source in ["(00001000 x)", "(10101000 x)"] {
            let lowered = lowered_one(source);
            assert!(matches!(lowered.kind, ExprKind::Call(_, _)));
        }
    }
}

#[cfg(test)]
mod exact_lowering_isolation_tests {
    use super::*;

    fn lower_current(source: &str) -> Expr {
        let parsed = crate::parser::parse_exact_domain_source(source)
            .expect("explicit exact-width source");
        lower_exact_domain_program(&parsed)
            .into_iter()
            .next()
            .expect("one expression")
    }

    #[test]
    fn historical_def_surface_cannot_steal_exact_d4_binder() {
        let current = lower_current("(def x (001 1))");
        let ExprKind::List(items) = current.kind else {
            panic!("legacy-only name has no implicit D4 law");
        };
        assert!(matches!(&items[0].kind, ExprKind::Symbol(name) if &**name == "def"));
        assert!(matches!(&items[2].kind, ExprKind::DomainCall(CoreDomainIdentity::D3(_), _)));
    }

    #[test]
    fn exact_d4_lambda_and_d3_quote_preserve_their_widths() {
        let current = lower_current("(0010 (x) (001 (00000001 x)))");
        let ExprKind::DomainCall(CoreDomainIdentity::D4(word), arguments) = current.kind else {
            panic!("exact D4 lambda");
        };
        assert_eq!(word.word().packed_bits(), 0b0010);
        let ExprKind::DomainCall(CoreDomainIdentity::D3(quote), quoted) = &arguments[1].kind else {
            panic!("D3 quote");
        };
        assert_eq!(quote.word().packed_bits(), 0b001);
        assert!(matches!(&quoted[0].kind, ExprKind::List(items)
            if matches!(&items[0].kind, ExprKind::DomainIdentity(identity)
                if identity.width() == 8 && identity.packed_bits() == 0b00000001)));
    }

    #[test]
    fn exact_lowering_remains_idempotent_with_nested_calls() {
        let parsed = crate::parser::parse_exact_domain_source(
            "(111 (100 (001 000)) (011 (001 000)))"
        ).unwrap();
        let lowered = lower_exact_domain_program(&parsed);
        assert_eq!(lower_exact_domain_program(&lowered), lowered);
    }
}
