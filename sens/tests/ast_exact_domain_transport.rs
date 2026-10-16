//! Незалежні свідки поточної доменної AST без збірки чужих unit-тестів.

use sens::{
    Bija3, Bit3, Bit4, Bit8, Bit9, BinaryNumber, CoreD4, CoreD8, CoreD9,
    DomainIdentity, Expr, ExprKind, Exactness, Span,
};
use sens::syntax::{
    decode_exact_fasl_program, decode_exact_wire_program,
    encode_exact_fasl_program, encode_exact_wire_program,
};

fn expression(kind: ExprKind) -> Expr {
    Expr { kind, span: Span::default() }
}

#[test]
fn точна_ширина_домену_не_зливається_за_однаковими_бітами() {
    let program = [
        expression(ExprKind::DomainIdentity(DomainIdentity::D3(
            Bija3::from_word(Bit3::new(0b001).unwrap()),
        ))),
        expression(ExprKind::DomainIdentity(DomainIdentity::D4(
            CoreD4::from_word(Bit4::new(0b0001).unwrap()),
        ))),
        expression(ExprKind::DomainIdentity(DomainIdentity::D8(
            CoreD8::from_word(Bit8::new(0b10101000).unwrap()),
        ))),
        expression(ExprKind::DomainIdentity(DomainIdentity::D9(
            CoreD9::from_word(Bit9::new(0b1_00000001).unwrap()),
        ))),
    ];

    let encoded = encode_exact_wire_program(&program).expect("доменний wire");
    let decoded = decode_exact_wire_program(&encoded).expect("доменний wire decode");
    assert_eq!(decoded.len(), program.len());
    for (actual, expected) in decoded.iter().zip(program.iter()) {
        assert_eq!(actual.kind, expected.kind);
    }
    assert_ne!(decoded[0].kind, decoded[1].kind);
    let hash = [17u8; 32];
    let encoded = encode_exact_fasl_program(&program, &hash).expect("доменний FASL");
    let (decoded, actual_hash) = decode_exact_fasl_program(&encoded).expect("доменний FASL decode");
    assert_eq!(actual_hash, hash);
    for (actual, expected) in decoded.iter().zip(program.iter()) {
        assert_eq!(actual.kind, expected.kind);
    }
}

#[test]
fn історичний_w8_під_цитатою_не_стає_d8() {
    let quoted = expression(ExprKind::List(vec![
        expression(ExprKind::DomainIdentity(DomainIdentity::D3(
            Bija3::from_word(Bit3::new(0b001).unwrap()),
        ))),
        expression(ExprKind::Pair(
            std::rc::Rc::new(expression(ExprKind::String("дані".into()))),
            std::rc::Rc::new(expression(ExprKind::Sid(
                sens::sens!(10101000),
            ))),
        )),
    ].into()));
    assert!(encode_exact_wire_program(std::slice::from_ref(&quoted)).is_none());
    assert!(encode_exact_fasl_program(std::slice::from_ref(&quoted), &[0; 32]).is_none());
    let old_wire = sens::wire_encode_program(std::slice::from_ref(&quoted));
    assert!(sens::wire_decode_program(&old_wire).is_some());
    assert!(decode_exact_wire_program(&old_wire).is_none());
    let old_fasl = sens::fasl_encode_program(&[quoted], &[0; 32]);
    assert!(sens::fasl_decode_program(&old_fasl).is_some());
    assert!(decode_exact_fasl_program(&old_fasl).is_none());
}

#[test]
fn тільки_d24_або_більше_є_числом_у_новому_форматі() {
    let number = expression(ExprKind::Number(2.0, Exactness::Exact));
    assert!(encode_exact_wire_program(&[number]).is_none());
    let raw = expression(ExprKind::BinaryNumber(BinaryNumber::from_bits([1, 0, 1]).unwrap()));
    assert!(encode_exact_wire_program(&[raw]).is_none());
    let bits = vec![0u8; 24];
    let qualified = expression(ExprKind::BinaryNumber(
        BinaryNumber::from_numeric_domain_bits(24, &bits).unwrap(),
    ));
    let encoded = encode_exact_wire_program(std::slice::from_ref(&qualified)).unwrap();
    let decoded = decode_exact_wire_program(&encoded).unwrap();
    assert_eq!(decoded[0].kind, qualified.kind);
}

#[test]
fn відсутній_d8_механізм_не_стає_виконуваним() {
    let d8 = sens::CoreDomainIdentity::D8(
        CoreD8::from_word(Bit8::new(0b10101000).unwrap()),
    );
    let data = expression(ExprKind::DomainIdentity(d8.into()));
    assert!(encode_exact_wire_program(&[data]).is_some());
    let call = expression(ExprKind::DomainCall(d8, [].into()));
    assert!(encode_exact_wire_program(&[call.clone()]).is_none());
    assert!(encode_exact_fasl_program(&[call], &[0; 32]).is_none());
}

#[test]
fn точний_ast_wire_fasl_d24_d5_sens_d1_з_раціональним_наслідком() {
    use sens::{Bit5, CoreD5, CoreDomainIdentity, ErrorKind, Session, Value};
    use sens::{
        decode_packed_numeric_value, encode_packed_numeric_value, eval_parsed_expressions,
    };

    // Код операції завжди є ратифікованою п'ятибітною координатою D5.
    let d5 = |bits: u8, args: Vec<Expr>| -> Expr {
        let identity = CoreDomainIdentity::D5(
            CoreD5::from_word(Bit5::new(bits).expect("точна ширина D5")),
        );
        let mut items = Vec::with_capacity(args.len() + 1);
        items.push(expression(ExprKind::DomainIdentity(identity.into())));
        items.extend(args);
        expression(ExprKind::List(items.into()))
    };

    // Вхідне D24 слово вже має явний домен, а не голий числовий літерал.
    let d24 = |payload: &str| -> Expr {
        assert!(!payload.is_empty() && payload.len() <= 24);
        let mut word = vec![0u8; 24 - payload.len()];
        word.extend(payload.bytes().map(|bit| match bit {
            b'0' => 0,
            b'1' => 1,
            _ => panic!("тільки точні двійкові цифри"),
        }));
        expression(ExprKind::BinaryNumber(
            BinaryNumber::from_numeric_domain_bits(24, &word)
                .expect("явний ратифікований D24"),
        ))
    };
    let quotient = |denominator: &str| {
        d5(0b10111, vec![d24("1"), d24(denominator)])
    };
    let sum = |denominator: &str| {
        d5(0b01010, vec![quotient("11"), quotient(denominator)])
    };
    let half = || quotient("10");

    // Обидва фізичні AST-контейнери вже існують. Жодного нового
    // числового opcode, десяткового посередника або T5-декодера тут немає.
    let run = |id: &str, code: Expr| -> Result<Value, sens::LanguageError> {
        let original = vec![code];
        let wire = encode_exact_wire_program(&original).expect("точні AST wire-байти");
        let decoded = decode_exact_wire_program(&wire).expect("точний wire decode");
        assert_eq!(decoded[0].kind, original[0].kind, "id={id} layer=transport wire");

        let source_hash = [0b10101010u8; 32];
        let fasl = encode_exact_fasl_program(&original, &source_hash)
            .expect("точні AST FASL-байти");
        let (fasl_program, actual_hash) = decode_exact_fasl_program(&fasl)
            .expect("точний FASL decode");
        assert_eq!(actual_hash, source_hash, "id={id} layer=transport hash");
        assert_eq!(fasl_program[0].kind, original[0].kind, "id={id} layer=transport fasl");

        let mut session = Session::default();
        let wire_result = eval_parsed_expressions(&decoded, &mut session)?;
        let mut other_session = Session::default();
        let fasl_result = eval_parsed_expressions(&fasl_program, &mut other_session)?;
        assert_eq!(
            wire_result.value, fasl_result.value,
            "REGRESSION id={id} layer=eval: wire/FASL SENS results differ; \
             reproduce=cargo test --locked -p sens --test ast_exact_domain_transport \
             точний_ast_wire_fasl_d24_d5_sens_d1_з_раціональним_наслідком -- --nocapture"
        );
        Ok(wire_result.value)
    };

    // SENS сам визначає D1:YES/NO точним порівнянням двох раціональних
    // результатів. Мутант змінює лише останній біт 110 → 111.
    for (id, predicate, expected) in [
        ("equal-not-less", d5(0b11010, vec![sum("110"), half()]), false),
        ("equal-not-greater", d5(0b11011, vec![sum("110"), half()]), false),
        ("one-bit-mutant-less", d5(0b11010, vec![sum("111"), half()]), true),
    ] {
        let actual = run(id, predicate).expect("двійкове D5 виконання");
        assert_eq!(
            actual.as_predicate_bit(), Some(expected),
            "REGRESSION id={id} layer=eval expected=D1:{expected} actual={actual:?}; \
             reproduce=cargo test --locked -p sens --test ast_exact_domain_transport \
             точний_ast_wire_fasl_d24_d5_sens_d1_з_раціональним_наслідком -- --nocapture"
        );
    }

    // Результат точного ділення / та від'ємного віднімання виводиться
    // самим SENS. Лише ПІСЛЯ цього наявний окремий механізм кадрування
    // пакує його біти. Це не граматика фізичного T5 .sens.
    for (id, program) in [
        ("ratio-result", quotient("11")),
        ("negative-result", d5(0b01011, vec![d24("1"), d24("11")])),
        ("sum-ratio-result", sum("110")),
    ] {
        let result = run(id, program).expect("допущена точна математика");
        assert!(matches!(&result, Value::NumericComposition(_)), "id={id}");
        let packed = encode_packed_numeric_value(&result).expect("наявний кадр D24+");
        let recovered = decode_packed_numeric_value(&packed)
            .expect("канонічне відновлення NEG/RATIO");
        assert_eq!(recovered, result, "id={id} layer=transport exact-value identity");
        let (mut bytes, bit_len) = packed.into_parts();
        if bit_len % 8 != 0 {
            *bytes.last_mut().unwrap() |= 1;
            assert!(
                sens::PackedBitstream::from_parts(bytes, bit_len).is_none(),
                "id={id}: неканонічне доповнення має бути відхилене"
            );
        }
    }

    // Замкненість від мінімальних математичних засновків: тут на вході
    // ТІЛЬКИ D24(0) і D24(1). -1, 2, -2, 1/2, -1/2 виводяться
    // бінарними функціями D5, а не потрапляють готовими значеннями.
    // AST-дерево лишається точним свідком шляхів операцій;
    // це ще НЕ сертифікат DERIVED_FACT світової моделі.
    let нуль = || d24("0");
    let один = || d24("1");
    let два = || d5(0b01010, vec![один(), один()]);
    let мінус_один = || d5(0b01011, vec![нуль(), один()]);
    let мінус_два = || d5(0b01011, vec![мінус_один(), один()]);
    let половина = || d5(0b10111, vec![один(), два()]);
    let мінус_половина = || d5(0b01011, vec![нуль(), половина()]);

    for (id, lhs, rhs) in [
        ("seed-derived-minus-two-less-minus-one", мінус_два(), мінус_один()),
        ("seed-derived-minus-one-less-minus-half", мінус_один(), мінус_половина()),
        ("seed-derived-minus-half-less-zero", мінус_половина(), нуль()),
        ("seed-derived-zero-less-half", нуль(), половина()),
        ("seed-derived-half-less-one", половина(), один()),
        ("seed-derived-one-less-two", один(), два()),
    ] {
        // Усі перевірки порядку виконують саме D5:11010 і D1 SENS.
        // Зовнішній тест не обчислює арифметику вдруге.
        let actual = run(id, d5(0b11010, vec![lhs, rhs]))
            .expect("SENS D5 повинен вивести порядок засновків");
        assert_eq!(
            actual.as_predicate_bit(), Some(true),
            "REGRESSION id={id} layer=eval expected=D1:YES actual={actual:?}; \
             reproduce=cargo test --locked -p sens --test ast_exact_domain_transport \
             точний_ast_wire_fasl_d24_d5_sens_d1_з_раціональним_наслідком -- --nocapture"
        );
    }

    // 1 + (-1) = 0 доводимо двома відсутніми строгими порядками,
    // щоб не вдавати, що D3 EQ є математичною рівністю дробів.
    for (id, opcode) in [
        ("seed-derived-zero-not-less", 0b11010),
        ("seed-derived-zero-not-greater", 0b11011),
    ] {
        let zero_from_seed = d5(0b01010, vec![один(), мінус_один()]);
        let actual = run(id, d5(opcode, vec![zero_from_seed, нуль()]))
            .expect("SENS D5 точна рівність нуля");
        assert_eq!(
            actual.as_predicate_bit(), Some(false),
            "REGRESSION id={id} layer=eval expected=D1:NO actual={actual:?}"
        );
    }

    // Заборонені числові операнди не отримують допуску навіть після
    // фізичного exact AST wire/FASL round-trip.
    for (id, source, kind) in [
        ("zero-divisor", d5(0b10111, vec![d24("1"), d24("0")]), ErrorKind::DivisionByZero),
        (
            "mixed-width",
            d5(0b10111, vec![
                d24("1"),
                expression(ExprKind::BinaryNumber(
                    BinaryNumber::from_numeric_domain_bits(
                        48, &{ let mut bits = vec![0u8;48]; bits[47] = 1; bits }
                    ).expect("явний D48"),
                )),
            ]),
            ErrorKind::Type,
        ),
    ] {
        let actual = run(id, source).expect_err("заборонена математична форма");
        assert_eq!(
            actual.kind, kind,
            "REGRESSION id={id} layer=eval expected={kind:?} actual={actual:?}"
        );
    }

    // Ніякого обходу через історичне Number: точний AST відмовляє раніше.
    let unqualified = d5(
        0b10111,
        vec![d24("1"), expression(ExprKind::Number(1.0, Exactness::Exact))],
    );
    assert!(encode_exact_wire_program(&[unqualified.clone()]).is_none());
    assert!(encode_exact_fasl_program(&[unqualified], &[0; 32]).is_none());
}
