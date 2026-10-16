//! Independent, reproducible Core numeric-domain arithmetic acceptance suite.
//! Mechanism inputs are full-width binary Dn words. Host u128 is used ONLY
//! as an independent test oracle, never as SENS numeric identity.
use sens::{BinaryNumber, Value, Session, eval_program};

fn d(width: usize, number: u128) -> BinaryNumber {
    assert!(width == 24 || width == 48);
    assert!(number < (1u128 << width));
    let bits = (0..width)
        .map(|i| ((number >> (width - 1 - i)) & 1) as u8)
        .collect::<Vec<_>>();
    BinaryNumber::from_numeric_domain_bits(width, &bits).unwrap()
}

fn magnitude_u128(number: &BinaryNumber) -> u128 {
    number.as_bits().iter().fold(0u128, |n, bit| (n << 1) | u128::from(*bit))
}

fn next(seed: &mut u64) -> u64 {
    // Fixed deterministic pseudorandom witness stream; no external crates.
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    *seed
}

#[test]
fn independent_oracle_18000_cases_of_d24_d48_exact_binary_arithmetic() {
    let mut seed = 0xA53C_1967_1B97_F00Du64;
    for (width, trials) in [(24usize, 12000usize), (48, 6000)] {
        let mask = (1u128 << width) - 1;
        for _ in 0..trials {
            let a = u128::from(next(&mut seed)) & mask;
            let b = u128::from(next(&mut seed)) & mask;
            let aa = d(width, a);
            let bb = d(width, b);
            let add = aa.add_admitted_numeric(&bb).unwrap();
            assert_eq!(magnitude_u128(&add), a + b);
            assert_eq!(add.numeric_domain_width(),
                Some(if a + b > mask { 2 * width } else { width }));
            assert_eq!(add.exact_numeric_domain_bits().unwrap().len(),
                add.numeric_domain_width().unwrap());

            let mul = aa.mul_admitted_numeric(&bb).unwrap();
            assert_eq!(magnitude_u128(&mul), a * b);
            assert_eq!(mul.numeric_domain_width(),
                Some(if a * b > mask { 2 * width } else { width }));
            assert_eq!(mul.exact_numeric_domain_bits().unwrap().len(),
                mul.numeric_domain_width().unwrap());

            let sub = aa.checked_sub_admitted_numeric(&bb);
            assert_eq!(sub.as_ref().map(magnitude_u128),
                a.checked_sub(b));
            if let Some(result) = &sub {
                assert_eq!(result.numeric_domain_width(), Some(width));
            }
            let (negative, abs) = aa.signed_difference_parts_admitted_numeric(&bb).unwrap();
            assert_eq!(negative, a < b);
            assert_eq!(magnitude_u128(&abs), a.abs_diff(b));
            assert_eq!(abs.numeric_domain_width(), Some(width));

            if b == 0 {
                assert!(aa.div_rem_admitted_numeric(&bb).is_none());
            } else {
                let (q, r) = aa.div_rem_admitted_numeric(&bb).unwrap();
                assert_eq!(magnitude_u128(&q), a / b);
                assert_eq!(magnitude_u128(&r), a % b);
                assert_eq!(q.numeric_domain_width(), Some(width));
                assert_eq!(r.numeric_domain_width(), Some(width));
                assert!(magnitude_u128(&r) < b);
            }
        }
    }
}

#[test]
fn entire_d24_d48_d96_d192_d384_ladder_keeps_all_bits_on_maximum_inputs() {
    for width in [24usize, 48, 96, 192, 384] {
        let max = BinaryNumber::from_numeric_domain_bits(width, &vec![1; width]).unwrap();
        let zero = BinaryNumber::from_numeric_domain_bits(width, &vec![0; width]).unwrap();
        let mut one_word = vec![0; width];
        one_word[width - 1] = 1;
        let one = BinaryNumber::from_numeric_domain_bits(width, &one_word).unwrap();
        let added = max.add_admitted_numeric(&one).unwrap();
        assert_eq!(added.numeric_domain_width(), Some(2*width));
        let mut expected_sum = vec![0; 2*width];
        expected_sum[width - 1] = 1; // 2^width (MSB-first)
        assert_eq!(added.exact_numeric_domain_bits().unwrap(), expected_sum);

        let squared = max.mul_admitted_numeric(&max).unwrap();
        assert_eq!(squared.numeric_domain_width(), Some(2*width));
        let mut expected_square = vec![0; 2*width];
        expected_square[..width - 1].fill(1);
        expected_square[2*width - 1] = 1; // (2^n - 1)^2
        assert_eq!(squared.exact_numeric_domain_bits().unwrap(), expected_square);
        assert_eq!(max.checked_sub_admitted_numeric(&max), Some(zero.clone()));
        assert_eq!(max.div_rem_admitted_numeric(&max), Some((one.clone(), zero.clone())));
        assert_eq!(zero.mul_admitted_numeric(&max), Some(zero.clone()));
        assert!(max.div_rem_admitted_numeric(&zero).is_none());
    }
}

#[test]
fn domain_inference_and_mixed_width_coercion_remain_forbidden() {
    let d24 = d(24, 17);
    let d48 = d(48, 17);
    assert_ne!(d24, d48);
    for pair in [(d24.clone(), d48.clone()), (d48.clone(), d24.clone()),
        (d24.clone(), BinaryNumber::from_bits([1,0,0,0,1]).unwrap())] {
        let (a,b) = pair;
        assert!(a.add_admitted_numeric(&b).is_none());
        assert!(a.mul_admitted_numeric(&b).is_none());
        assert!(a.checked_sub_admitted_numeric(&b).is_none());
        assert!(a.div_rem_admitted_numeric(&b).is_none());
    }
}

#[test]
fn real_core_session_runs_natural_math_and_fails_closed_on_unencoded_composites() {
    let mut session = Session::default();
    session.environment.define("a", Value::BinaryNumber(d(24, 9)));
    session.environment.define("b", Value::BinaryNumber(d(24, 3)));
    for (expr, expected) in [("(+ a b)", 12), ("(* a b)", 27),
        ("(- a b)", 6), ("(/ a b)", 3)] {
        let got = eval_program(expr, &mut session).expect("Core D24 math").value;
        let Value::BinaryNumber(ref number) = got else { panic!("must be D24"); };
        assert_eq!(number.numeric_domain_width(), Some(24), "{expr}");
        assert_eq!(magnitude_u128(number), expected, "{expr}");
    }

    session.environment.define("a", Value::BinaryNumber(d(24, 2)));
    session.environment.define("b", Value::BinaryNumber(d(24, 3)));
    for (expr, expected) in [("(- a b)", "-1"), ("(/ a b)", "2/3")] {
        let result = eval_program(expr, &mut session)
            .expect("exact NEG/RATIO composition must execute").value;
        assert_eq!(result.to_string(), expected, "{expr}");
    }
    let a = eval_program("(- a b)", &mut session).unwrap().value;
    let q = eval_program("(/ a b)", &mut session).unwrap().value;
    session.environment.define("negative", a);
    session.environment.define("ratio", q);
    assert_eq!(eval_program("(+ negative b)", &mut session).unwrap().value,
        Value::BinaryNumber(d(24, 2)));
    assert_eq!(eval_program("(* ratio b)", &mut session).unwrap().value,
        Value::BinaryNumber(d(24, 2)));
    session.environment.define("b", Value::BinaryNumber(d(24, 0)));
    assert_eq!(eval_program("(/ a b)", &mut session).unwrap_err().kind,
        sens::ErrorKind::DivisionByZero);
}
