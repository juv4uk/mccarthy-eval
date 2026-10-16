//! Exact mechanisms for the first admitted D6 arithmetic residents.
//!
//! Semantic authority: #3393 / Contract 11.8.
//! Mechanism authority: #3394.
//!
//! ADD1/SUB1 do not mint a new arithmetic backend. They are admitted as
//! LOWER_DOMAIN_COMPOSITION over the already-admitted D5 PLUS/DIFFERENCE laws
//! with an exact ONE in the SAME admitted D24+ domain as the operand.

use crate::{
    BinaryNumber, Bit5, CoreD5, CoreDomainIdentity, Environment, ErrorKind, LanguageError, Span, Value,
};

fn d5(bits: u8) -> CoreDomainIdentity {
    CoreDomainIdentity::D5(CoreD5::from_word(
        Bit5::new(bits).expect("D5 arithmetic coordinate"),
    ))
}

pub(super) fn invoke(
    identity: CoreDomainIdentity,
    args: &[Value],
    environment: &Environment,
    span: Span,
) -> Option<Result<Value, LanguageError>> {
    let CoreDomainIdentity::D6(word) = identity else {
        return None;
    };

    let bits = word.word().packed_bits();
    if !matches!(bits, 0b001110 | 0b001111) {
        return None;
    }

    if args.len() != 1 {
        return Some(Err(LanguageError::new(
            ErrorKind::Arity,
            format!(
                "{identity}: exact D6 ADD1/SUB1 mechanism expects 1 argument; received {}",
                args.len()
            ),
            span,
        )));
    }

    // The old implementation inserted an unqualified Rational(1), silently
    // crossing the D24+ admission boundary. ADD1/SUB1 are operators in D6,
    // but their numeric *operands* must be explicitly qualified D24+ values.
    let width = match &args[0] {
        Value::BinaryNumber(number) => number.numeric_domain_width(),
        Value::NumericComposition(composition) => Some(composition.domain_width()),
        _ => None,
    };
    let Some(width) = width else {
        return Some(Err(LanguageError::new(
            ErrorKind::Type,
            "D6 ADD1/SUB1 requires a D24+ numeric operand",
            span,
        )));
    };
    let mut one_bits = vec![0; width];
    one_bits[width - 1] = 1;
    let one = BinaryNumber::from_numeric_domain_bits(width, &one_bits)
        .expect("D24+ domain width from qualified operand");
    let composed_args = [args[0].clone(), Value::BinaryNumber(one)];
    let d5_identity = match bits {
        0b001110 => d5(0b01010), // ADD1 = D5 PLUS(x, 1)
        0b001111 => d5(0b01011), // SUB1 = D5 DIFFERENCE(x, 1)
        _ => unreachable!(),
    };

    Some(
        super::d5_arithmetic::invoke(d5_identity, &composed_args, environment, span)
            .expect("selected D5 arithmetic resident has an admitted mechanism"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d24(n: u32) -> Value {
        let mut bits = vec![0; 24];
        for i in 0..24 {
            bits[23 - i] = ((n >> i) & 1) as u8;
        }
        Value::BinaryNumber(
            BinaryNumber::from_numeric_domain_bits(24, &bits).unwrap()
        )
    }

    #[test]
    fn add1_carries_d24_into_d48_and_sub1_uses_ratified_neg_composition() {
        let env = Environment::root();
        let add1 = CoreDomainIdentity::D6(crate::CoreD6::from_word(
            crate::Bit6::new(0b001110).unwrap(),
        ));
        let sub1 = CoreDomainIdentity::D6(crate::CoreD6::from_word(
            crate::Bit6::new(0b001111).unwrap(),
        ));

        // A full D24 natural + 1 cannot wrap or invent an unqualified result.
        let carried = invoke(add1, &[d24((1 << 24) - 1)], &env, Span::default())
            .unwrap().unwrap();
        let Value::BinaryNumber(ref number) = carried else {
            panic!("D24 carry must stay an exact numeric domain word");
        };
        assert_eq!(number.numeric_domain_width(), Some(48));
        let mut next_bits = vec![0; 48];
        next_bits[23] = 1; // full-width D48 encoding of 2^24
        assert_eq!(number.exact_numeric_domain_bits().unwrap(), next_bits);

        // After a D24 carry, ADD1 must continue in the new exact D48 domain.
        // The successor must not become a source literal, wrap, or stall.
        let resumed = invoke(add1, &[carried.clone()], &env, Span::default())
            .unwrap().unwrap();
        let Value::BinaryNumber(ref resumed_number) = resumed else {
            panic!("D48 ADD1 after carry must remain a numeric-domain value");
        };
        assert_eq!(resumed_number.numeric_domain_width(), Some(48));
        let mut advanced_bits = next_bits.clone();
        advanced_bits[47] = 1; // 2^24 + 1, not a D24 or unqualified literal
        assert_eq!(resumed_number.exact_numeric_domain_bits().unwrap(), advanced_bits);

        // 0 - 1 is canonical NEG(D24(1)), not a wrapped 24-bit word
        // and not a host Rational with unqualified identity.
        let negative = invoke(sub1, &[d24(0)], &env, Span::default())
            .unwrap().unwrap();
        let Value::NumericComposition(ref composition) = negative else {
            panic!("D24 SUB1 below zero must return canonical NEG");
        };
        assert_eq!(composition.domain_width(), 24);
        assert_eq!(negative.to_string(), "-1");
        assert_eq!(
            invoke(add1, &[negative], &env, Span::default()).unwrap().unwrap(),
            d24(0),
        );
    }

    #[test]
    fn add1_sub1_use_same_d24_numeric_domain_not_legacy_one() {
        let env = Environment::root();
        for (bits, expected) in [(0b001110, d24(8)), (0b001111, d24(6))] {
            let identity = CoreDomainIdentity::D6(crate::CoreD6::from_word(
                crate::Bit6::new(bits).unwrap(),
            ));
            let actual = invoke(identity, &[d24(7)], &env, Span::default())
                .unwrap().unwrap();
            assert_eq!(actual, expected);
            let blocked = invoke(
                identity,
                &[Value::Rational(crate::Rational::integer(7))],
                &env,
                Span::default(),
            ).unwrap().unwrap_err();
            assert_eq!(blocked.kind, ErrorKind::Type);
        }
    }
}
