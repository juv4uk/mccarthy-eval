//! Exact source-shaped program-data representation.
//!
//! Converts already-lowered SENS AST into ordinary runtime data without
//! reintroducing human spellings or legacy SID8 identity.
//! DomainCall(identity,args...) becomes a proper list headed by the exact
//! first-class DomainIdentity. This module owns representation only.

use crate::{DomainIdentity, ErrorKind, Expr, ExprKind, LanguageError, Span, Value};
use std::rc::Rc;

fn invalid_program_data(message: impl Into<String>) -> LanguageError {
    LanguageError::new(ErrorKind::InvalidForm, message, Span::default())
}

/// Convert one already-lowered expression into source-shaped exact program-data.
pub fn expr_to_exact_program_data(expr: &Expr) -> Result<Value, LanguageError> {
    Ok(match &expr.kind {
        // Тут формується КАНОНІЧНА програма для компілятора, а не
        // історичне Lisp-значення. Числова семантика належить лише D24+.
        // Не допускаємо неявного виведення домену з десяткового тексту,
        // раціонального літерала чи голих бітів.
        ExprKind::Number(_, _) | ExprKind::Rational(_) | ExprKind::NumericBuffer(_) => {
            return Err(invalid_program_data(
                "точнодоменна програма: історичні числові літерали не мають D24+",
            ));
        }
        ExprKind::BinaryNumber(value) if value.numeric_domain_width().is_some() => {
            Value::BinaryNumber(value.clone())
        }
        ExprKind::BinaryNumber(_) => {
            return Err(invalid_program_data(
                "точнодоменна програма: двійкове число потребує явного домену D24+",
            ));
        }
        ExprKind::DomainIdentity(identity) => Value::DomainIdentity(*identity),
        ExprKind::String(value) => Value::String(value.clone()),
        ExprKind::Symbol(value) => Value::Symbol(value.clone()),
        ExprKind::List(items) => Value::list(
            items
                .iter()
                .map(expr_to_exact_program_data)
                .collect::<Result<Vec<_>, _>>()?,
        ),
        ExprKind::Pair(head, tail) => Value::Pair(
            Rc::new(expr_to_exact_program_data(head)?),
            Rc::new(expr_to_exact_program_data(tail)?),
        ),
        ExprKind::DomainCall(identity, arguments) => {
            let mut items = Vec::with_capacity(arguments.len() + 1);
            items.push(Value::DomainIdentity(DomainIdentity::from(*identity)));
            for argument in arguments.iter() {
                items.push(expr_to_exact_program_data(argument)?);
            }
            Value::list(items)
        }
        ExprKind::Sid(_) | ExprKind::Call(_, _) => {
            return Err(invalid_program_data(
                "exact program-data must not contain legacy Sid/Call nodes",
            ));
        }
        ExprKind::Local { .. } => {
            return Err(invalid_program_data(
                "source-shaped exact program-data must not contain resolved Local nodes",
            ));
        }
    })
}

/// Convert an already-lowered top-level program into a proper data list.
pub fn lowered_program_to_exact_data(program: &[Expr]) -> Result<Value, LanguageError> {
    Ok(Value::list(
        program
            .iter()
            .map(expr_to_exact_program_data)
            .collect::<Result<Vec<_>, _>>()?,
    ))
}

#[cfg(test)]
mod тести_міграції_чисел_d24 {
    use super::*;
    use crate::{Bija3, BinaryNumber, Bit3, CoreDomainIdentity, Exactness};

    fn вузол(kind: ExprKind) -> Expr {
        Expr {
            kind,
            span: Span::default(),
        }
    }

    fn кваліфіковане_число(width: usize) -> Expr {
        let mut bits = vec![0u8; width];
        bits[width - 1] = 1;
        вузол(ExprKind::BinaryNumber(
            BinaryNumber::from_numeric_domain_bits(width, &bits)
                .expect("доведене числове слово D24+"),
        ))
    }

    #[test]
    fn рівне_значення_зберігає_відмінні_домени_d24_d48() {
        let d24 = expr_to_exact_program_data(&кваліфіковане_число(24)).unwrap();
        let d48 = expr_to_exact_program_data(&кваліфіковане_число(48)).unwrap();
        assert_ne!(d24, d48, "D24 та D48 не тотожні через спільне значення");
        assert!(matches!(d24, Value::BinaryNumber(ref n) if n.numeric_domain_width() == Some(24)));
        assert!(matches!(d48, Value::BinaryNumber(ref n) if n.numeric_domain_width() == Some(48)));
    }

    #[test]
    fn заборонено_історичні_числові_літерали_без_d24() {
        let старі = [
            вузол(ExprKind::Number(1.0, Exactness::Exact)),
            вузол(ExprKind::Rational(
                crate::Rational::from_literal("1", "2").expect("історичний літерал"),
            )),
            вузол(ExprKind::BinaryNumber(BinaryNumber::parse("1").unwrap())),
        ];
        for старе in старі {
            assert_eq!(
                expr_to_exact_program_data(&старе).unwrap_err().kind,
                ErrorKind::InvalidForm,
            );
        }
    }

    #[test]
    fn вкладені_числа_не_відмиваються_через_списки_й_доменні_виклики() {
        let старе = вузол(ExprKind::Number(3.0, Exactness::Exact));
        let список = вузол(ExprKind::List(Rc::from([старе.clone()])));
        assert!(expr_to_exact_program_data(&список).is_err());
        let пара = вузол(ExprKind::Pair(Rc::new(кваліфіковане_число(24)), Rc::new(старе.clone())));
        assert!(expr_to_exact_program_data(&пара).is_err());
        let quote = CoreDomainIdentity::D3(
            Bija3::from_word(Bit3::new(0b001).unwrap()),
        );
        let виклик = вузол(ExprKind::DomainCall(quote, Rc::from([старе])));
        assert!(expr_to_exact_program_data(&виклик).is_err());
    }

    #[test]
    fn точний_доменний_виклик_з_числом_d24_залишається_програмою() {
        let quote = CoreDomainIdentity::D3(
            Bija3::from_word(Bit3::new(0b001).unwrap()),
        );
        let source = вузол(ExprKind::DomainCall(quote, Rc::from([кваліфіковане_число(24)])));
        let result = expr_to_exact_program_data(&source).expect("доведена форма");
        assert!(matches!(result, Value::Pair(_, _)));
    }
}
