//! Contract 11.9 / owner 2026-10-12: binary words are not numbers.
//! Only exact D24+ qualified values may enter mathematical mechanisms.
//! These witnesses use real Core evaluation, not an auxiliary arithmetic
//! simulator, and do not admit any new source syntax or numeric literal.

use sens::{
    eval_parsed_expressions, eval_program, parse_mixed_exact_domain, BinaryNumber,
    BinarySourceWord, Bit1, Bit2, Bit3, Bit4, Bit5, Bit6, Bit7, Bit8, Bit9,
    DomainIdentity, ErrorKind, Exactness, Rational, Session, Value,
};

fn numeric(width: usize, magnitude: u32) -> Value {
    let mut bits = vec![0; width];
    for i in 0..width {
        if i < 32 {
            bits[width - 1 - i] = ((magnitude >> i) & 1) as u8;
        }
    }
    Value::BinaryNumber(
        BinaryNumber::from_numeric_domain_bits(width, &bits)
            .expect("D24+ qualified numeric word"),
    )
}

fn exact_call(source: &str, session: &mut Session) -> Result<Value, sens::LanguageError> {
    let forms = parse_mixed_exact_domain(source)?;
    eval_parsed_expressions(&forms, session).map(|result| result.value)
}

#[test]
fn all_representable_non_numeric_domain_words_are_rejected_by_math() {
    let words = [
        BinarySourceWord::W1(Bit1::new(1).unwrap()),
        BinarySourceWord::W2(Bit2::new(1).unwrap()),
        BinarySourceWord::W3(Bit3::new(1).unwrap()),
        BinarySourceWord::W4(Bit4::new(1).unwrap()),
        BinarySourceWord::W5(Bit5::new(1).unwrap()),
        BinarySourceWord::W6(Bit6::new(1).unwrap()),
        BinarySourceWord::W7(Bit7::new(1).unwrap()),
        BinarySourceWord::W8(Bit8::new(1).unwrap()),
        BinarySourceWord::W9(Bit9::new(1).unwrap()),
    ];
    let mut session = Session::default();
    for word in words {
        let identity = DomainIdentity::from_source_word(word);
        session.environment.define("left", Value::DomainIdentity(identity));
        session.environment.define("right", Value::DomainIdentity(identity));

        // Exact D5 operation identities; the arithmetic entry point must
        // refuse the operands rather than reinterpret their packed bits.
        for source in [
            "(01010 left right)", "(01011 left right)",
            "(10110 left right)", "(10111 left right)",
            "(11010 left right)", "(11011 left right)",
        ] {
            let error = exact_call(source, &mut session)
                .expect_err("D1-D9 identity must not become an arithmetic operand");
            assert_eq!(error.kind, ErrorKind::Type, "D{}: {source}", identity.width());
            assert!(error.message.contains("D24+"), "D{}: {error}", identity.width());
        }
        for source in ["(001110 left)", "(001111 left)"] {
            let error = exact_call(source, &mut session)
                .expect_err("D6 successor/predecessor requires a D24+ operand");
            assert_eq!(error.kind, ErrorKind::Type, "D{}: {source}", identity.width());
            assert!(error.message.contains("D24+"), "D{}: {error}", identity.width());
        }
    }
}

#[test]
fn bare_numbers_and_numeric_spelling_cannot_bypass_d24_admission() {
    let mut session = Session::default();
    for (name, value) in [
        ("legacy-float", Value::Number(1.0, Exactness::Exact)),
        ("legacy-ratio", Value::Rational(Rational::integer(1))),
        ("unqualified-bits", Value::BinaryNumber(BinaryNumber::one())),
    ] {
        session.environment.define("left", value.clone());
        session.environment.define("right", value);
        for source in [
            "(01010 left right)", "(10110 left right)",
            "(10111 left right)", "(11010 left right)",
        ] {
            let error = exact_call(source, &mut session).unwrap_err();
            assert_eq!(error.kind, ErrorKind::Type, "{name}: {source}");
            assert!(error.message.contains("D24+"), "{name}: {error}");
        }
    }
    // Source digit strings without a D24+ domain remain non-mathematical,
    // independent of their length; the reader cannot grant domain identity.
    for source in [
        "(+ 1 1)",
        "(+ 100 100)",
        "(+ 0000000001 0000000001)",
        "(* 24 48)",
        // A 24/48-bit-looking source spelling is not a D24/D48
        // qualified value. Width alone has no numeric-domain authority.
        "(+ 000000000000000000000001 000000000000000000000001)",
    ] {
        let error = eval_program(source, &mut session).unwrap_err();
        assert_eq!(error.kind, ErrorKind::Type, "{source}");
        assert!(error.message.contains("D24+"), "{source}: {error}");
    }
}

#[test]
fn d5_d6_operator_codes_compute_without_literals_across_d24_d384() {
    let mut session = Session::default();
    for width in [24usize, 48, 96, 192, 384] {
        session.environment.define("left", numeric(width, 7));
        session.environment.define("right", numeric(width, 2));
        for (source, expected) in [
            ("(01010 left right)", 9),
            ("(01011 left right)", 5),
            ("(10110 left right)", 14),
            ("(001110 left)", 8),
            ("(001111 left)", 6),
        ] {
            assert_eq!(
                exact_call(source, &mut session).unwrap(),
                numeric(width, expected),
                "D{width}: {source}"
            );
        }
        assert_eq!(
            exact_call("(11010 right left)", &mut session).unwrap(),
            Value::predicate_bit(true),
        );
        assert_eq!(
            exact_call("(11011 right left)", &mut session).unwrap(),
            Value::predicate_bit(false),
        );
        assert_eq!(
            exact_call("(10111 left right)", &mut session).unwrap().to_string(),
            "7/2",
        );
    }
}

#[test]
fn d1_predicates_and_d3_eq_remain_non_arithmetic_laws() {
    let mut session = Session::default();
    session.environment.define("left", Value::predicate_bit(true));
    session.environment.define("right", Value::predicate_bit(false));

    // D3 EQ tests exact atom identity; it is not numeric '='.
    assert_eq!(
        exact_call("(101 left left)", &mut session).unwrap(),
        Value::predicate_bit(true),
    );
    assert_eq!(
        exact_call("(101 left right)", &mut session).unwrap(),
        Value::predicate_bit(false),
    );
    assert_eq!(
        exact_call("(010 left)", &mut session).unwrap(),
        Value::predicate_bit(true),
    );

    // The same D1 operands MUST be rejected by mathematical ordering.
    let error = exact_call("(11010 left right)", &mut session).unwrap_err();
    assert_eq!(error.kind, ErrorKind::Type);
    assert!(error.message.contains("D24+"), "{error}");
}

#[test]
fn ten_bit_domain_ratification_does_not_invent_d10_callability_or_arithmetic() {
    let mut session = Session::default();
    session.environment.define("number", numeric(24, 9));

    // D10 has a whole-domain owner ratification, but no generic W10
    // runtime callable lift, per-resident proof or physical T5 admission.
    // An ordinary ten-digit head must NOT be reinterpreted as a D10
    // callable merely because its binary spelling fits ten bits.
    for expression in ["(0000000001 number)", "(1111111111 number)"] {
        let result = exact_call(expression, &mut session);
        assert!(result.is_err(), "{expression} must fail closed before D10 admission");
    }

    // Nor can a ten-digit spelling be a substitute for a qualified
    // D24+ number. The value's own domain is never inferred from digits.
    let error = exact_call("(01010 number 0000000001)", &mut session).unwrap_err();
    assert_eq!(error.kind, ErrorKind::Type);
    assert!(error.message.contains("D24+"), "{error}");
}

/// Prove the admissible replacement for Core4 length-onto's old
/// (01010 acc 1), without changing pinned Core4 source or its FASL before a
/// verified source+binary rebuild. Only the caller supplies an exact D24 seed.
/// The recursive implementation is SENS-owned D3/D4/D6 source.
#[test]
fn lisp_owned_d24_length_fold_uses_ratified_d6_add1_without_number_literals() {
    const SOURCE: &str = r#"
        (0011 довжина-д24-до
          (0010 (values acc)
            (110
              ((0101 values) acc)
              ((0100 (010 values))
                (довжина-д24-до (011 values) (001110 acc))))))
    "#;

    let mut session = Session::bare();
    let definitions = parse_mixed_exact_domain(SOURCE)
        .expect("typed D3/D4/D6 call heads in Lisp-owned numeric fold");
    eval_parsed_expressions(&definitions, &mut session)
        .expect("admitted Lisp-owned recursive fold definition");

    for count in 0u32..=9 {
        let mut list = Value::Nil;
        for _ in 0..count {
            list = Value::Pair(
                std::rc::Rc::new(Value::Symbol(std::rc::Rc::from("елемент"))),
                std::rc::Rc::new(list),
            );
        }
        session.environment.define("items", list);
        session.environment.define("seed", numeric(24, 0));

        // The length is computed by domain-owned ADD1, not by a numeric
        // source literal or implicit D1/D3/D5 -> D24 coercion.
        let result = eval_program("(довжина-д24-до items seed)", &mut session)
            .expect("exact D24 Lisp-owned fold");
        assert_eq!(result.value, numeric(24, count), "length={count}");
    }

    // Crossing the D24 boundary in a real Lisp-owned recursive call must
    // carry the exact result into D48, without truncating to 24 bits.
    let unit = Value::Symbol(std::rc::Rc::from("елемент"));
    session.environment.define(
        "items",
        Value::Pair(
            std::rc::Rc::new(unit.clone()),
            std::rc::Rc::new(Value::Pair(
                std::rc::Rc::new(unit),
                std::rc::Rc::new(Value::Nil),
            )),
        ),
    );
    session.environment.define(
        "seed",
        Value::BinaryNumber(
            BinaryNumber::from_numeric_domain_bits(24, &vec![1; 24])
                .expect("exact maximum D24 word"),
        ),
    );
    let carried = eval_program("(довжина-д24-до items seed)", &mut session)
        .expect("Lisp-owned fold must retain carry into D48").value;
    // D24(max) is 2^24 - 1. Two Lisp-owned ADD1 steps yield 2^24 + 1,
    // represented losslessly in D48; expecting 2^24 + 2 would be off by one.
    assert_eq!(carried, numeric(48, (1 << 24) + 1));
    
    // An unqualified seed never becomes D24 just because it is called 0.
    session.environment.define(
        "items",
        Value::Pair(
            std::rc::Rc::new(Value::Symbol(std::rc::Rc::from("елемент"))),
            std::rc::Rc::new(Value::Nil),
        ),
    );
    session.environment.define("seed", Value::Rational(Rational::integer(0)));
    let err = eval_program("(довжина-д24-до items seed)", &mut session)
        .expect_err("unqualified zero must not enter D6 ADD1");
    assert_eq!(err.kind, ErrorKind::Type);
    assert!(err.message.contains("D24+"), "{err}");
}

/// A second Core4 migration slice: nth's old numeric (01011 i 1)
/// can become the already-admitted D6 SUB1. Equality with zero is D3 EQ
/// over *explicitly* qualified values, not mathematical comparison of
/// a legacy scalar literal. This is a candidate law witness, not a
/// modification of pinned Core4 or a newly admitted D24 source literal.
#[test]
fn lisp_owned_d24_nth_uses_d3_identity_and_d6_sub1_without_number_literals() {
    const SOURCE: &str = r#"
        (0011 елемент-д24
          (0010 (i xs zero)
            (110
              ((101 i zero) (100 xs))
              ((0100 (101 i zero))
                (елемент-д24 (001111 i) (011 xs) zero)))))
    "#;
    let mut session = Session::bare();
    let definitions = parse_mixed_exact_domain(SOURCE)
        .expect("D3 equality and D6 SUB1 remain distinct identities");
    eval_parsed_expressions(&definitions, &mut session)
        .expect("Lisp-owned D24 nth recursive definition");

    let mut list = Value::Nil;
    for symbol in ["третій", "другий", "перший"] {
        list = Value::Pair(
            std::rc::Rc::new(Value::Symbol(std::rc::Rc::from(symbol))),
            std::rc::Rc::new(list),
        );
    }
    session.environment.define("xs", list);
    session.environment.define("zero", numeric(24, 0));

    for (index, answer) in ["перший", "другий", "третій"].iter().enumerate() {
        session.environment.define("i", numeric(24, index as u32));
        let actual = eval_program("(елемент-д24 i xs zero)", &mut session)
            .expect("D24 index accepted through D3 EQ / D6 SUB1").value;
        assert_eq!(actual, Value::Symbol(std::rc::Rc::from(*answer)));
    }

    // A same-looking unqualified zero is NOT D24(0); it must never act
    // as an index in the arithmetic decrement branch.
    session.environment.define("i", Value::Rational(Rational::integer(1)));
    let error = eval_program("(елемент-д24 i xs zero)", &mut session)
        .expect_err("unqualified index refused by exact D6 SUB1");
    assert_eq!(error.kind, ErrorKind::Type);
    assert!(error.message.contains("D24+"), "{error}");
}

#[test]
fn different_numeric_domains_do_not_implicitly_coerce() {
    let mut session = Session::default();
    session.environment.define("left", numeric(24, 7));
    session.environment.define("right", numeric(48, 7));
    for source in [
        "(01010 left right)", "(10110 left right)",
        "(10111 left right)", "(11010 left right)",
    ] {
        let error = exact_call(source, &mut session).unwrap_err();
        assert_eq!(error.kind, ErrorKind::Type, "{source}");
    }
}
