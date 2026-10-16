//! Production mechanism for the ratified CAR/CDR selector generator law.
//!
//! Semantic authority: #3202 / #2055.
//!
//! Current proved family:
//! - D3 root 100 -> CAR
//! - D3 root 011 -> CDR
//! - selector-family suffix 0 -> compose CAR
//! - selector-family suffix 1 -> compose CDR
//! - D4 admits one selector suffix bit
//! - owner-ratified D5 v2 (#3305/#3331) admits two selector suffix bits
//! - owner-ratified D6 (#3393) admits three selector suffix bits
//!
//! This law is family-local. It does not create a universal suffix meaning.
//! D7/D8 remain outside this production selector decoder and fail closed.

use super::special_forms;
use crate::{CoreDomainIdentity, ErrorKind, LanguageError, Span, Value};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Step {
    Car,
    Cdr,
}

/// Decode only the exact-width, ratified selector shape. No temporary
/// instruction array: the suffix bits already encode the execution order.
fn decode(identity: CoreDomainIdentity) -> Option<(Step, u8, usize)> {
    let width = identity.width();
    if !(3..=6).contains(&width) {
        return None;
    }

    let payload = identity.packed_bits();
    let root = match payload >> (width - 3) {
        0b100 => Step::Car,
        0b011 => Step::Cdr,
        _ => return None,
    };
    Some((root, payload, width - 3))
}

#[inline]
fn select(step: Step, value: &Value, span: Span) -> Result<Value, LanguageError> {
    match step {
        Step::Car => special_forms::car_value(value, span),
        Step::Cdr => special_forms::cdr_value(value, span),
    }
}

pub(super) fn invoke(
    identity: CoreDomainIdentity,
    args: &[Value],
    span: Span,
) -> Option<Result<Value, LanguageError>> {
    let (root, payload, suffix_len) = decode(identity)?;

    if args.len() != 1 {
        return Some(Err(LanguageError::new(
            ErrorKind::Arity,
            format!(
                "{identity}: exact selector law expects 1 argument; received {}",
                args.len()
            ),
            span,
        )));
    }

    // Lisp selector composition executes the rightmost step first. Those
    // steps are precisely the low-order payload bits; the D3 root runs last.
    // The first step borrows the argument directly instead of allocating an
    // unnecessary clone of the complete input list.
    let first = if suffix_len == 0 {
        root
    } else if payload & 1 == 0 {
        Step::Car
    } else {
        Step::Cdr
    };
    let mut value = match select(first, &args[0], span) {
        Ok(value) => value,
        Err(error) => return Some(Err(error)),
    };
    for shift in 1..suffix_len {
        let step = if (payload >> shift) & 1 == 0 {
            Step::Car
        } else {
            Step::Cdr
        };
        value = match select(step, &value, span) {
            Ok(value) => value,
            Err(error) => return Some(Err(error)),
        };
    }
    if suffix_len != 0 {
        value = match select(root, &value, span) {
            Ok(value) => value,
            Err(error) => return Some(Err(error)),
        };
    }
    Some(Ok(value))
}
