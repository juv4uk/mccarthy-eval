//! Conservative executable-code admissibility witness for Frame3 candidates.
//!
//! IMPORTANT: "not proved impossible" never means semantically valid.
//! This only rejects *provably* impossible statically-headed D3 call arities.
//! Quoted syntax is data, not an executable call; COND and dynamic heads
//! are not statically excluded without an independent authoritative law.
//! This module changes neither the canonical reader nor the physical codec.

use crate::syntax::{Expr, ExprKind};
use crate::DomainIdentity;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdmissionWitness {
    ProvablyImpossible,
    NotProvenImpossible,
}

/// This is a safe negative witness, NOT an executable program validator.
///
/// D3: 001 QUOTE, 010 ATOM, 011 CDR, 100 CAR, 101 EQ, 111 CONS.
/// QUOTE does not evaluate its one data argument. Arguments of the other
/// five known fixed-arity primitives must be executable expressions.
/// COND=110 and dynamic/application forms require separate rules.
pub fn check_executable_code(expr: &Expr) -> AdmissionWitness {
    if impossible(expr) {
        AdmissionWitness::ProvablyImpossible
    } else {
        AdmissionWitness::NotProvenImpossible
    }
}

/// Негативний доказ для обох форм виклику: до та після точного
/// D3 lowering. Спільний закон не залежить від історичного W8-коду.
fn impossible(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::DomainCall(crate::CoreDomainIdentity::D3(head), arguments) => {
            impossible_d3_fixed_arity(head.word().packed_bits(), arguments)
        }
        ExprKind::List(items) => {
            let Some((head, arguments)) = items.split_first() else {
                return false; // () — значення, а не порожній виклик.
            };

            // Обчислювана голова з гарантовано неможливою формою
            // робить зовнішній виклик неможливим.
            if matches!(&head.kind, ExprKind::List(_) | ExprKind::DomainCall(_, _))
                && impossible(head)
            {
                return true;
            }
            let ExprKind::DomainIdentity(DomainIdentity::D3(identity)) = &head.kind else {
                return false; // Динамічну голову або інший домен не вгадуємо.
            };
            impossible_d3_fixed_arity(identity.word().packed_bits(), arguments)
        }
        _ => false, // Інші домени та дані — поза вузьким доказом.
    }
}

/// Від'ємний свідок застосовний лише там, де арність D3 уже встановлена.
/// Неретифікована виконуваність, D3:000 та COND — не підміна доказу.
fn impossible_d3_fixed_arity(bits: u8, arguments: &[Expr]) -> bool {
    let expected = match bits {
        0b001..=0b100 => 1,
        0b101 | 0b111 => 2,
        _ => return false, // D3:000 та 110 COND не мають тут fixed-arity доказу.
    };
    if arguments.len() != expected {
        return true;
    }
    if bits == 0b001 {
        return false; // QUOTE: аргумент є даними, не виконується.
    }
    arguments.iter().any(impossible)
}

#[cfg(test)]
mod exact_d3_frame3_admission_tests {
    use super::{check_executable_code, AdmissionWitness};
    use crate::domain_ladder::DomainCoordinate;
    use crate::{Bija3, Bit1, Bit3, CoreDomainIdentity, DomainIdentity, PredicateBit};
    use crate::syntax::{Expr, ExprKind, Span};
    use std::rc::Rc;

    fn expr(kind: ExprKind) -> Expr {
        Expr { kind, span: Span::default() }
    }

    fn value() -> Expr {
        expr(ExprKind::DomainIdentity(DomainIdentity::D1(
            PredicateBit::from_word(Bit1::new(0).unwrap()),
        )))
    }

    fn d3(bits: u8) -> Bija3 {
        Bija3::from_word(Bit3::new(bits).unwrap())
    }

    fn lowered(bits: u8, args: Vec<Expr>) -> Expr {
        expr(ExprKind::DomainCall(CoreDomainIdentity::D3(d3(bits)), Rc::from(args)))
    }

    fn source_list(bits: u8, args: Vec<Expr>) -> Expr {
        let mut items = vec![expr(ExprKind::DomainIdentity(DomainIdentity::D3(d3(bits))))];
        items.extend(args);
        expr(ExprKind::List(items.into()))
    }

    #[test]
    fn всі_2046_точних_координат_зберігають_ширину_без_семантичного_ліфта() {
        let mut count = 0;
        let mut semantic_bits = 0;
        for width in 1u8..=10 {
            for bits in 0..(1u16 << width) {
                let coordinate = DomainCoordinate::new(width, bits).unwrap();
                let visible = coordinate.видимі_біти();
                assert_eq!(visible.len(), usize::from(width));
                assert_eq!(
                    DomainCoordinate::з_видимих_бітів(&visible),
                    Some(coordinate),
                    "втрачено точну координату D{width}:{visible}"
                );
                // Такий самий payload, але інша ширина — інша identity.
                if width > 1 && bits < (1u16 << (width - 1)) {
                    assert_ne!(
                        DomainCoordinate::new(width - 1, bits),
                        Some(coordinate),
                        "хибна міждоменна тотожність D{width}:{visible}"
                    );
                }
                count += 1;
                semantic_bits += usize::from(width);
            }
        }
        assert_eq!(count, 2046);
        assert_eq!(semantic_bits, 18434);
        assert!(DomainCoordinate::new(0, 0).is_none());
        assert!(DomainCoordinate::new(11, 0).is_none());
        assert!(DomainCoordinate::new(10, 1024).is_none());
        // Перебрані адреси — не сертифікат виконання 2046 функцій.
    }

    #[test]
    fn точна_d3_арність_ідентична_для_source_list_та_lowered_domain_call() {
        let dead = AdmissionWitness::ProvablyImpossible;
        let unknown = AdmissionWitness::NotProvenImpossible;
        for bits in 0u8..8 {
            for count in 0..=3 {
                let args = vec![value(); count];
                let expected = match bits {
                    0b001..=0b100 if count != 1 => dead,
                    0b101 | 0b111 if count != 2 => dead,
                    _ => unknown,
                };
                assert_eq!(
                    check_executable_code(&source_list(bits, args.clone())),
                    expected,
                    "старий source D3:{bits:03b}, args={count}"
                );
                assert_eq!(
                    check_executable_code(&lowered(bits, args)),
                    expected,
                    "lowered D3:{bits:03b}, args={count}"
                );
            }
        }
    }

    #[test]
    fn quote_захищає_дані_а_виконувана_гілка_відхиляє_помилку() {
        let invalid_inner = lowered(0b010, vec![]); // ATOM вимагає 1 аргумент.
        let quoted = lowered(0b001, vec![invalid_inner.clone()]);
        let executable = lowered(0b010, vec![invalid_inner]);
        assert_eq!(
            check_executable_code(&quoted),
            AdmissionWitness::NotProvenImpossible
        );
        assert_eq!(
            check_executable_code(&executable),
            AdmissionWitness::ProvablyImpossible
        );
        // COND не отримує вигаданої фіксованої арності.
        assert_eq!(
            check_executable_code(&lowered(0b110, vec![executable])),
            AdmissionWitness::NotProvenImpossible
        );
    }
}
