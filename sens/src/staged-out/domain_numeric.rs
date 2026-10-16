//! Exact arithmetic for mathematical NEG/RATIO compositions over existing D24+
//! natural atoms. This is a Core VALUE mechanism, NOT a new SENS domain.
//! Domain identities remain explicit, with no host floats, mixed-domain
//! coercion, decimal literals, or invented binary semantic opcodes.
use crate::{BinaryNumber, Value};
use std::cmp::Ordering;

/// Executable canonical compositions of admitted D24/D48/... natural atoms.
///
/// Both constituent numeric atoms always occupy the exact SAME admitted Dn.
/// This guarantees unique sign and reduced rational shape *within* that
/// numeric domain, without taking any of its 2^n natural payloads.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DomainNumericComposition {
    // Private: only canonical() may construct these. Public enum variants
    // would allow callers to forge NEG(0), denominator=0 or unreduced RATIO.
    negative: bool,
    numerator: BinaryNumber,
    denominator: Option<BinaryNumber>, // None precisely for negative integer
}

impl DomainNumericComposition {
    pub fn domain_width(&self) -> usize {
        self.numerator.numeric_domain_width().expect("admitted domain")
    }

    pub fn as_parts(&self) -> (bool, &BinaryNumber, Option<&BinaryNumber>) {
        (self.negative, &self.numerator, self.denominator.as_ref())
    }
}

fn normalized(a: &BinaryNumber) -> BinaryNumber {
    BinaryNumber::from_bits(a.as_bits().to_vec()).expect("admitted binary natural")
}

fn own_word(n: BinaryNumber, width: usize) -> Option<BinaryNumber> {
    if n.width() > width || !BinaryNumber::is_numeric_domain_width(width) {
        return None;
    }
    let mut bits = vec![0; width - n.width()];
    bits.extend_from_slice(n.as_bits());
    BinaryNumber::from_numeric_domain_bits(width, &bits)
}

/// Mathematical canonicalization. Invariants:
/// NEG(0) and RATIO(0,q) -> original domain-qualified zero;
/// RATIO(p,1) -> integer / NEG(integer);
/// otherwise gcd(num,den)=1, den>1, and no negative denominator.
fn canonical(
    negative: bool,
    numerator: BinaryNumber,
    denominator: BinaryNumber,
    minimum_width: usize,
) -> Option<Value> {
    if denominator.is_zero() || !BinaryNumber::is_numeric_domain_width(minimum_width) {
        return None;
    }
    let n = normalized(&numerator);
    let d = normalized(&denominator);
    let common = n.gcd(&d);
    let (n, nr) = n.div_rem(&common)?;
    let (d, dr) = d.div_rem(&common)?;
    if !nr.is_zero() || !dr.is_zero() {
        return None;
    }
    // Recognize the reduced unit before constructing an explicit Dn word.
    // BinaryNumber currently normalizes padded leading zeros, so checking
    // after own_word() also recognizes 1; doing it here avoids allocating
    // an entire temporary Dn denominator that integer/zero cases discard.
    let denominator_is_one = d.as_bits() == [1];

    let mut width = minimum_width;
    while n.width().max(d.width()) > width {
        width = width.checked_mul(2)?;
    }
    let n = own_word(n, width)?;
    if n.is_zero() { return Some(Value::BinaryNumber(n)); }
    if denominator_is_one {
        if negative {
            return Some(Value::NumericComposition(DomainNumericComposition {
                negative: true, numerator: n, denominator: None,
            }));
        }
        return Some(Value::BinaryNumber(n));
    }
    // Only an actual fraction needs a fully qualified Dn denominator.
    let d = own_word(d, width)?;
    Some(Value::NumericComposition(DomainNumericComposition {
        negative, numerator: n, denominator: Some(d),
    }))
}

fn ingredients(value: &Value) -> Option<(usize, bool, BinaryNumber, BinaryNumber)> {
    match value {
        Value::BinaryNumber(n) => {
            let width = n.numeric_domain_width()?;
            Some((width, false, normalized(n), BinaryNumber::one()))
        }
        Value::NumericComposition(c) => {
            let width = c.domain_width();
            Some((
                width, c.negative, normalized(&c.numerator),
                c.denominator.as_ref().map_or_else(BinaryNumber::one, normalized),
            ))
        }
        _ => None,
    }
}

/// Construct exact signed/rational Dn Value, preserving one base Dn or
/// growing only when its arithmetic result actually needs more bits.
pub fn canonical_parts(
    negative: bool,
    numerator: &BinaryNumber,
    denominator: &BinaryNumber,
) -> Option<Value> {
    let width = numerator.numeric_domain_width()?;
    if denominator.numeric_domain_width() != Some(width) { return None; }
    canonical(negative, normalized(numerator), normalized(denominator), width)
}

/// Fast, non-semantic Dn admission. No rational cross-product, host magnitude
/// conversion, or shared binary prefix may stand in for the exact domain tag.
/// This does not compare mathematical values or authorize mixed-Dn coercion.
pub fn numeric_value_domain_width(value: &Value) -> Option<usize> {
    match value {
        Value::BinaryNumber(n) => n.numeric_domain_width(),
        Value::NumericComposition(c) => Some(c.domain_width()),
        _ => None,
    }
}

pub fn shared_numeric_domain_width(lhs: &Value, rhs: &Value) -> Option<usize> {
    let width = numeric_value_domain_width(lhs)?;
    (numeric_value_domain_width(rhs) == Some(width)).then_some(width)
}

pub fn is_domain_value(v: &Value) -> bool {
    numeric_value_domain_width(v).is_some()
}

/// Exactly two operands in the SAME Dn; arithmetic creates no unqualified
/// identity. The specific result-domain growth is explicit and validated.
pub fn compute(operator: &str, lhs: &Value, rhs: &Value) -> Option<Value> {
    // Натуральні слова D24+ уже мають доведені власні бітові механізми.
    // Не проводимо їх через зайві NEG/RATIO gcd та повторне означення Dn.
    // Допуск SAME-Dn і перенесення Dn -> D(2n) перевіряє BinaryNumber.
    // Некваліфіковані й змішані Dn мають відхилятися тут, без fallback.
    if let (Value::BinaryNumber(left), Value::BinaryNumber(right)) = (lhs, rhs) {
        match operator {
            "+" => return left.add_admitted_numeric(right).map(Value::BinaryNumber),
            "*" => return left.mul_admitted_numeric(right).map(Value::BinaryNumber),
            // A nonnegative natural difference is already a Dn atom.
            // Reuse the qualified bit mechanism instead of introducing
            // two unit denominators, cross-products, gcd and a ratio wrapper.
            // A negative difference still follows the original NEG path.
            "-" if left.numeric_domain_width().is_some()
                && left.numeric_domain_width() == right.numeric_domain_width()
                && left >= right =>
            {
                return left.checked_sub_admitted_numeric(right).map(Value::BinaryNumber);
            }
            _ => {}
        }
    }

    // Reject D24/D48 (or non-Dn) BEFORE cloning potentially wide rational
    // constituents into the private calculation.
    let width = shared_numeric_domain_width(lhs, rhs)?;
    let (_, x_neg, x_num, x_den) = ingredients(lhs)?;
    let (_, y_neg, y_num, y_den) = ingredients(rhs)?;

    let (negative, numerator, denominator) = match operator {
        "+" | "-" => {
            let y_effective_neg = if operator == "-" { !y_neg } else { y_neg };
            let xn = x_num.mul(&y_den);
            let yn = y_num.mul(&x_den);
            let d = x_den.mul(&y_den);
            let (neg, num) = if x_neg == y_effective_neg {
                (x_neg, xn.add(&yn))
            } else if xn >= yn {
                (x_neg, xn.checked_sub(&yn)?)
            } else {
                (y_effective_neg, yn.checked_sub(&xn)?)
            };
            (neg, num, d)
        }
        "*" => (x_neg ^ y_neg, x_num.mul(&y_num), x_den.mul(&y_den)),
        "/" => {
            if y_num.is_zero() { return None; }
            (x_neg ^ y_neg, x_num.mul(&y_den), x_den.mul(&y_num))
        }
        _ => return None,
    };
    canonical(negative, numerator, denominator, width)
}

pub fn unary(operator: &str, value: &Value) -> Option<Value> {
    let (width, negative, n, d) = ingredients(value)?;
    match operator {
        "-" => canonical(!negative, n, d, width),
        "/" if !n.is_zero() => canonical(negative, d, n, width),
        _ => None,
    }
}

/// Exact mathematical signed-rational compare under one explicit Dn.
/// The semantic identity in Value::eq remains domain-qualified and structural.
pub fn compare(operator: &str, lhs: &Value, rhs: &Value) -> Option<bool> {
    // Admitted natural values already carry their exact D24/D48/... identity.
    // Compare their normalized binary magnitudes directly: the generic
    // rational path below allocates four clones and two products needlessly.
    // Domain width is checked BEFORE comparing; equal naked bits in
    // different domains must never become comparable through this shortcut.
    if let (Value::BinaryNumber(left), Value::BinaryNumber(right)) = (lhs, rhs) {
        let width = left.numeric_domain_width()?;
        if right.numeric_domain_width() != Some(width) {
            return None;
        }
        let order = left.cmp(right);
        return Some(match operator {
            "=" => order == Ordering::Equal,
            "<" => order == Ordering::Less,
            ">" => order == Ordering::Greater,
            _ => return None,
        });
    }

    // A canonical negative Dn composition is smaller than every admitted
    // natural in the SAME domain, including zero. Do not construct temporary
    // rational denominators or multiply just to discover opposite signs.
    // This is a mechanism shortcut over already-ratified binary Dn values.
    match (lhs, rhs) {
        (Value::BinaryNumber(natural), Value::NumericComposition(signed))
            if signed.negative =>
        {
            if natural.numeric_domain_width() != Some(signed.domain_width()) {
                return None;
            }
            return Some(match operator {
                "=" => false,
                "<" => false,
                ">" => true,
                _ => return None,
            });
        }
        (Value::NumericComposition(signed), Value::BinaryNumber(natural))
            if signed.negative =>
        {
            if natural.numeric_domain_width() != Some(signed.domain_width()) {
                return None;
            }
            return Some(match operator {
                "=" => false,
                "<" => true,
                ">" => false,
                _ => return None,
            });
        }
        _ => {}
    }

    // The generic signed/rational comparison needs exact products only after
    // both operands have independently passed the domain-identity gate.
    shared_numeric_domain_width(lhs, rhs)?;
    let (_, x_neg, xn, xd) = ingredients(lhs)?;
    let (_, y_neg, yn, yd) = ingredients(rhs)?;
    let order = match (x_neg, y_neg) {
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        (left_neg, _) => {
            let cmp = xn.mul(&yd).cmp(&yn.mul(&xd));
            if left_neg { cmp.reverse() } else { cmp }
        }
    };
    Some(match operator {
        "=" => order == Ordering::Equal,
        "<" => order == Ordering::Less,
        ">" => order == Ordering::Greater,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(width: usize, u: u64) -> BinaryNumber {
        let mut bits = vec![0; width];
        for i in 0..width.min(64) { bits[width - 1 - i] = ((u >> i) & 1) as u8; }
        BinaryNumber::from_numeric_domain_bits(width, &bits).unwrap()
    }

    /// Mechanism regression: no numeric-domain widening, textual decimal
    /// intermediary or rational cross-products for two admitted naturals.
    #[test]
    fn natural_dn_comparison_retains_width_and_order_with_no_rational_bridge() {
        for width in [24usize, 48, 96, 192] {
            let zero = Value::BinaryNumber(d(width, 0));
            let two = Value::BinaryNumber(d(width, 2));
            let three = Value::BinaryNumber(d(width, 3));

            for (left, right, equal, less, greater) in [
                (&zero, &zero, true, false, false),
                (&two, &three, false, true, false),
                (&three, &two, false, false, true),
                (&three, &three, true, false, false),
            ] {
                for (operator, expected) in [
                    ("=", equal), ("<", less), (">", greater),
                ] {
                    let actual = compare(operator, left, right);
                    assert_eq!(
                        actual, Some(expected),
                        "test_id=natural-dn-compare layer=eval domain=D{width} op={operator} expected={expected} actual={actual:?}; reproduce=cargo test -p sens natural_dn_comparison_retains_width_and_order_with_no_rational_bridge"
                    );
                }
            }
            assert_eq!(compare("+", &two, &three), None);
            assert_eq!(
                compare("=", &two, &Value::BinaryNumber(d(width * 2, 2))),
                None,
                "D{width} and D{} cannot share a numeric identity",
                width * 2,
            );

            // Signed compositions must continue through the existing
            // rational/signed comparison mechanism, not this shortcut.
            let neg_two = canonical_parts(true, &d(width, 2), &d(width, 1))
                .expect("admitted signed Dn");
            assert_eq!(compare("<", &neg_two, &zero), Some(true));
            assert_eq!(compare("=", &neg_two, &two), Some(false));
        }
        let legacy = Value::BinaryNumber(BinaryNumber::one());
        let d24 = Value::BinaryNumber(d(24, 1));
        assert_eq!(compare("=", &legacy, &d24), None);
        assert_eq!(compare("=", &d24, &legacy), None);
    }

    #[test]
    fn exact_dn_admission_never_depends_on_rational_value_equality() {
        let unqualified = Value::BinaryNumber(BinaryNumber::one());
        for width in [24usize, 48, 96, 192, 384] {
            let one = Value::BinaryNumber(d(width, 1));
            let two = Value::BinaryNumber(d(width, 2));
            let fraction = canonical_parts(false, &d(width, 1), &d(width, 3))
                .expect("exact nonintegral fraction in Dn");
            let signed = canonical_parts(true, &d(width, 1), &d(width, 1))
                .expect("exact negative integer in Dn");
            let same_magnitude_doubled_width = Value::BinaryNumber(d(width * 2, 1));

            assert_eq!(numeric_value_domain_width(&fraction), Some(width));
            assert_eq!(numeric_value_domain_width(&signed), Some(width));
            for lhs in [&one, &fraction, &signed] {
                for rhs in [&two, &fraction, &signed] {
                    assert_eq!(shared_numeric_domain_width(lhs, rhs), Some(width),
                        "D{width}: admission depends only on exact Dn tags");
                }
                assert_eq!(shared_numeric_domain_width(lhs, &same_magnitude_doubled_width), None);
                assert_eq!(shared_numeric_domain_width(&same_magnitude_doubled_width, lhs), None);
                assert_eq!(shared_numeric_domain_width(lhs, &unqualified), None);
                assert_eq!(shared_numeric_domain_width(&unqualified, lhs), None);
            }
        }
    }

    #[test]
    fn signed_dn_vs_natural_is_ordered_without_rational_cross_products() {
        for width in [24usize, 48, 96, 192] {
            let zero = Value::BinaryNumber(d(width, 0));
            let two = Value::BinaryNumber(d(width, 2));
            let neg_fraction = canonical_parts(true, &d(width, 2), &d(width, 3))
                .expect("ratified negative Dn rational");
            let positive_fraction = canonical_parts(false, &d(width, 2), &d(width, 3))
                .expect("ratified positive Dn rational");
            for (op, expected_forward, expected_reverse) in [
                ("=", false, false),
                ("<", false, true),
                (">", true, false),
            ] {
                for natural in [&zero, &two] {
                    let forward = compare(op, natural, &neg_fraction);
                    let reverse = compare(op, &neg_fraction, natural);
                    assert_eq!(
                        forward, Some(expected_forward),
                        "test_id=dn-sign-shortcut layer=eval D{width} op={op} positive-versus-negative: expected={expected_forward}, actual={forward:?}; reproduce=cargo test -p sens signed_dn_vs_natural_is_ordered_without_rational_cross_products"
                    );
                    assert_eq!(
                        reverse, Some(expected_reverse),
                        "test_id=dn-sign-shortcut layer=eval D{width} op={op} negative-versus-positive: expected={expected_reverse}, actual={reverse:?}; reproduce=cargo test -p sens signed_dn_vs_natural_is_ordered_without_rational_cross_products"
                    );
                }
            }
            // The ordinary rational path must still handle *positive*
            // signed/rational compositions rather than the sign shortcut.
            assert_eq!(compare(">", &two, &positive_fraction), Some(true));
            assert_eq!(compare("<", &positive_fraction, &two), Some(true));
            assert_eq!(compare("=", &two, &positive_fraction), Some(false));
            let other_width = width * 2;
            let wrong = Value::BinaryNumber(d(other_width, 2));
            assert_eq!(compare("=", &neg_fraction, &wrong), None);
            assert_eq!(compare("=", &wrong, &neg_fraction), None);
            let unqualified = Value::BinaryNumber(BinaryNumber::one());
            assert_eq!(compare("=", &neg_fraction, &unqualified), None);
            assert_eq!(compare("=", &unqualified, &neg_fraction), None);
        }
    }

    #[test]
    fn composition_normalizes_sign_gcd_unit_and_zero() {
        let zero = Value::BinaryNumber(d(24, 0));
        assert_eq!(unary("-", &zero), Some(zero.clone()));
        assert_eq!(compute("/", &zero, &Value::BinaryNumber(d(24, 3))), Some(zero));
        assert_eq!(canonical_parts(false, &d(24, 2), &d(24, 4)),
            canonical_parts(false, &d(24, 1), &d(24, 2)));
        assert_eq!(canonical_parts(false, &d(24, 6), &d(24, 3)),
            Some(Value::BinaryNumber(d(24, 2))));
        assert_eq!(canonical_parts(true, &d(24, 5), &d(24, 1)),
            unary("-", &Value::BinaryNumber(d(24, 5))));
        assert!(canonical_parts(false, &d(24, 1), &d(24, 0)).is_none());
        assert!(canonical_parts(false, &d(24, 1), &d(48, 2)).is_none());
    }

    #[test]
    fn one_bit_denominator_mutation_keeps_d24_plus_integers_canonical() {
        // Targeted witness: the one-bit change 000...001 -> 000...011
        // is exactly what separates an integer from a rational.
        // Every intermediate constituent still belongs to the same Dn.
        for width in [24usize, 48, 96, 192] {
            let one = d(width, 1);
            let two = d(width, 2);
            let three = d(width, 3);
            let zero = d(width, 0);

            // as_bits() returns a normalized magnitude. The *domain word*
            // must be compared at its authoritative full Dn width.
            let one_bits = one.exact_numeric_domain_bits().expect("exact Dn(1)");
            let three_bits = three.exact_numeric_domain_bits().expect("exact Dn(3)");
            assert_eq!(one_bits.len(), width);
            assert_eq!(three_bits.len(), width);
            let bit_distance: usize = one_bits.iter()
                .zip(three_bits.iter())
                .map(|(a, b)| usize::from(a != b))
                .sum();
            assert_eq!(bit_distance, 1, "D{width}: single-bit mutation");

            let integer = canonical_parts(false, &two, &one)
                .expect("unit denominator reduces to admitted natural Dn");
            assert_eq!(integer, Value::BinaryNumber(two.clone()),
                "D{width}: RATIO(2,1) must not survive as an extra tag");

            let negative = canonical_parts(true, &two, &one)
                .expect("negative integer still uses exact Dn");
            let Value::NumericComposition(ref neg) = negative else {
                panic!("D{width}: a negative integer has its own composition")
            };
            let (is_negative, numerator, denominator) = neg.as_parts();
            assert!(is_negative);
            assert_eq!(neg.domain_width(), width);
            assert_eq!(numerator.numeric_domain_width(), Some(width));
            assert!(denominator.is_none(),
                "D{width}: no unneeded Dn denominator for integer -2");
            assert_eq!(negative.to_string(), "-2");

            let ratio = canonical_parts(false, &two, &three)
                .expect("one-bit-mutated denominator is a valid fraction");
            let Value::NumericComposition(ref fraction) = ratio else {
                panic!("D{width}: the mutated denominator is not unity")
            };
            assert_eq!(ratio.to_string(), "2/3");
            assert_eq!(fraction.domain_width(), width);
            let (_, numerator, denominator) = fraction.as_parts();
            assert_eq!(numerator.numeric_domain_width(), Some(width));
            assert_eq!(denominator.expect("actual fraction").numeric_domain_width(), Some(width));

            let divided = compute(
                "/", &Value::BinaryNumber(d(width, 6)),
                &Value::BinaryNumber(d(width, 3)),
            ).expect("same domain 6/3");
            assert_eq!(divided, Value::BinaryNumber(two),
                "D{width}: reduced division becomes exactly Dn(2)");
            assert_eq!(
                canonical_parts(true, &zero, &one),
                Some(Value::BinaryNumber(zero)),
                "D{width}: negative zero must not allocate a sign or denominator"
            );
            assert!(canonical_parts(
                false, &d(width, 2), &d(width * 2, 1)
            ).is_none(), "D{width}: mixed Dn values are forbidden");
        }
    }

    #[test]
    fn signed_rational_arithmetic_matches_independent_exact_i128_oracle() {
        let gcd = |mut a: i128, mut b: i128| -> i128 {
            while b != 0 { (a,b) = (b,a%b); }
            a.abs()
        };
        let mut seed = 0x1234_5678_9abc_def1u64;
        let mut next = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        for _ in 0..4096 {
            let a = (next() % 33) as i128;
            let b = (next() % 32 + 1) as i128;
            let c = (next() % 33) as i128;
            let d0 = (next() % 32 + 1) as i128;
            let left_negative = next() & 1 != 0;
            let right_negative = next() & 1 != 0;
            let left = canonical_parts(left_negative, &d(24,a as u64), &d(24,b as u64)).unwrap();
            let right = canonical_parts(right_negative, &d(24,c as u64), &d(24,d0 as u64)).unwrap();
            let x = if left_negative { -a } else { a };
            let y = if right_negative { -c } else { c };
            for op in ["+","-","*","/"] {
                let pair = match op {
                    "+" => (x*d0 + y*b, b*d0),
                    "-" => (x*d0 - y*b, b*d0),
                    "*" => (x*y, b*d0),
                    "/" if y != 0 => (x*d0, b*y),
                    "/" => { assert!(compute(op,&left,&right).is_none()); continue; }
                    _ => unreachable!(),
                };
                let (mut n,mut den) = pair;
                if den < 0 { n = -n; den = -den; }
                let common = gcd(n.abs(),den);
                let n = n/common;
                let den = den/common;
                let expected = if den == 1 { n.to_string() } else {
                    format!("{n}/{den}")
                };
                let got = compute(op, &left, &right).unwrap();
                assert_eq!(got.to_string(), expected, "input {x}/{b} {op} {y}/{d0}");
                if let Value::NumericComposition(c) = &got {
                    assert_eq!(c.domain_width(),24);
                    let (negative,num,denominator) = c.as_parts();
                    assert_eq!(negative,n<0);
                    assert!(!num.is_zero());
                    assert_eq!(num.numeric_domain_width(),Some(24));
                    if let Some(denominator) = denominator {
                        assert_eq!(denominator.numeric_domain_width(),Some(24));
                        assert!(denominator.as_bits() != [0] && denominator.as_bits() != [1]);
                    }
                    assert!(got.to_canonical_wire_string().contains("WIRE-UNSUPPORTED"));
                }
            }
            assert_eq!(compare("=", &left, &right), Some(x*d0 == y*b));
            assert_eq!(compare("<", &left, &right), Some(x*d0 < y*b));
        }
    }

    #[test]
    fn rational_high_bit_products_promote_components_from_d24_to_d48_without_loss() {
        let n = (1u64 << 24) - 1;
        let den = n - 1;
        let ratio = canonical_parts(false, &d(24,n), &d(24,den)).unwrap();
        assert_eq!(ratio.to_string(), format!("{n}/{den}"));
        let squared = compute("*",&ratio,&ratio).unwrap();
        let Value::NumericComposition(ref r) = squared else { panic!("exact RATIO") };
        assert_eq!(r.domain_width(),48);
        let (neg, numerator, denominator) = r.as_parts();
        assert!(!neg);
        let denominator = denominator.expect("reduced RATIO requires denominator");
        assert_eq!(numerator.numeric_domain_width(),Some(48));
        assert_eq!(denominator.numeric_domain_width(),Some(48));
        assert_eq!(numerator.to_decimal_string(), (n*n).to_string());
        assert_eq!(denominator.to_decimal_string(), (den*den).to_string());
        assert_eq!(squared.to_string(), format!("{}/{}",n*n,den*den));
        let negated = unary("-",&squared).unwrap();
        assert_eq!(negated.to_string(),format!("-{}/{}",n*n,den*den));
        assert_eq!(compute("+",&squared,&negated),Some(Value::BinaryNumber(
            BinaryNumber::from_numeric_domain_bits(48,&vec![0;48]).unwrap()
        )));
        // The original operand still belongs to D24. No generic D24 <-> D48 bridge.
        assert!(compute("+",&squared,&ratio).is_none());
    }

    #[test]
    fn composite_domain_is_not_inferred_and_overflow_retains_all_bits() {
        let a = Value::BinaryNumber(d(24, (1<<24)-1));
        let b = Value::BinaryNumber(d(24, 2));
        let result = compute("*",&a,&b).unwrap();
        let Value::BinaryNumber(ref bits) = result else { panic!("integer"); };
        assert_eq!(bits.numeric_domain_width(),Some(48));
        let neg = unary("-",&result).unwrap();
        assert_eq!(neg.to_string(), format!("-{}", ((1u64<<24)-1)*2));
        let Value::NumericComposition(ref composition) = neg else { panic!("negative"); };
        assert_eq!(composition.domain_width(),48);
        let other = Value::BinaryNumber(d(48,2));
        assert!(compute("+",&Value::NumericComposition(composition.clone()),&other).is_some());
        assert!(compute("+",&Value::NumericComposition(composition.clone()),&b).is_none());
        assert!(compare("=",&Value::NumericComposition(composition.clone()),&b).is_none());
    }

    #[test]
    fn one_bit_boundary_diagnostic_preserves_every_domain_bit() {
        // Незалежний оракул: (2^n - 2) + 1 вміщується в Dn;
        // (2^n - 1) + 1 = 2^n мусить бути точним D(2n).
        // Зміна ОДНОГО біта на вході змінює обраний домен результату.
        for width in [24usize, 48, 96, 192] {
            let mut nearly_full = vec![1u8; width];
            nearly_full[width - 1] = 0;
            let full = vec![1u8; width];
            let mut one = vec![0u8; width];
            one[width - 1] = 1;
            let admitted = |word: &[u8]| {
                Value::BinaryNumber(
                    BinaryNumber::from_numeric_domain_bits(width, word)
                        .expect("точна бітність D24+"),
                )
            };
            let almost = admitted(&nearly_full);
            let maximum = admitted(&full);
            let unit = admitted(&one);

            let fitting = compute("+", &almost, &unit).expect("вміщується в Dn");
            let Value::BinaryNumber(fitted) = &fitting else {
                panic!("ціле натуральне має залишитися доменним числом")
            };
            assert_eq!(fitted.numeric_domain_width(), Some(width), "D{width}: fit");
            assert_eq!(fitted.exact_numeric_domain_bits(), Some(full.clone()),
                "D{width}: усі n бітів суми повинні зберегтися");

            let crossing = compute("+", &maximum, &unit).expect("точне перенесення");
            let Value::BinaryNumber(carried) = &crossing else {
                panic!("carry мусить залишитися натуральним числом")
            };
            let mut expected = vec![0u8; 2 * width];
            expected[width - 1] = 1;
            assert_eq!(carried.numeric_domain_width(), Some(2 * width),
                "D{width}: не можна обрізати старший біт");
            assert_eq!(carried.exact_numeric_domain_bits(), Some(expected),
                "D{width}: єдиний біт перенесення на точній D(2n) позиції");

            let unchanged = compute("*", &maximum, &unit)
                .expect("множення на одиницю того самого домену");
            assert_eq!(unchanged, maximum, "D{width}: natural MUL path");
            let mut two = vec![0u8; width];
            two[width - 2] = 1;
            let doubled = compute("*", &maximum, &admitted(&two))
                .expect("точне множення з перенесенням");
            let Value::BinaryNumber(product) = &doubled else {
                panic!("добуток має залишитися точним натуральним словом")
            };
            let mut product_bits = vec![0u8; 2 * width];
            product_bits[(width - 1)..(2 * width - 1)].fill(1);
            assert_eq!(product.numeric_domain_width(), Some(2 * width));
            assert_eq!(product.exact_numeric_domain_bits(), Some(product_bits));

            let legacy_one = Value::BinaryNumber(BinaryNumber::one());
            for op in ["+", "*"] {
                assert!(compute(op, &maximum, &legacy_one).is_none(),
                    "D{width}: {op} must reject unqualified one");
                assert!(compute(op, &legacy_one, &maximum).is_none(),
                    "D{width}: {op} must reject unqualified first operand");
                let other_domain = Value::BinaryNumber(
                    BinaryNumber::from_numeric_domain_bits(2 * width, &vec![0; 2 * width])
                        .unwrap(),
                );
                assert!(compute(op, &maximum, &other_domain).is_none(),
                    "D{width}: {op} must reject mixed Dn/D(2n)");
            }
        }
    }

    #[test]
    fn arithmetic_closes_over_negative_and_exact_rational_values() {
        let one = Value::BinaryNumber(d(24, 1));
        let two = Value::BinaryNumber(d(24, 2));
        let three = Value::BinaryNumber(d(24, 3));
        let half = compute("/", &one, &two).unwrap();
        let third = compute("/", &one, &three).unwrap();
        assert_eq!(compute("+", &half, &half), Some(one.clone()));
        let sixth = compute("/", &half, &three).unwrap();
        assert_eq!(compute("+", &third, &sixth), Some(half.clone()));
        let negative = compute("-", &one, &three).unwrap();
        assert_eq!(compute("+", &negative, &two), Some(Value::BinaryNumber(d(24, 0))));
        assert_eq!(compute("*", &negative, &half), unary("-", &one));
        assert_eq!(compute("/", &one, &half), Some(two));
        assert_eq!(compare("<", &negative, &half), Some(true));
        assert_eq!(compare("=", &half, &half), Some(true));
        assert_eq!(compare(">", &third, &half), Some(false));
        assert!(compute("/", &one, &Value::BinaryNumber(d(24, 0))).is_none());
    }
}
