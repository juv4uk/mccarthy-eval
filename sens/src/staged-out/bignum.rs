//! A small, hand-rolled arbitrary-precision signed integer, used only by
//! `Rational` (see `value.rs`). Deliberately not a crate dependency: this
//! crate stays at zero dependencies (see docs/language-core.md's
//! capability-free contract), and "Rust doing the low-level numeric
//! algorithm it's exceptionally good at" doesn't require pulling in a
//! third-party implementation of it — schoolbook algorithms, kept simple
//! and correct, not optimized for cryptographic-scale numbers this
//! language has no use for.
//! Malenke, napysane vruchnu tsile chyslo dovilnoi tochnosti zi znakom,
//! vykorystovuietsia lyshe `Rational` (dyv. `value.rs`). Navmysno ne
//! crate-zalezhnist: tsei kreit lyshaietsia na nulovii kilkosti
//! zalezhnostei (dyv. capability-free kontrakt u docs/language-core.md), a
//! "Rust, shcho robyt nyzkorivnevyi chyslovyi alhorytm, u yakomu vin
//! osoblyvo sylnyi" ne vymahaie pidtiahuvannia storonoi realizatsii —
//! shkilni alhorytmy, prosti y korektni, ne optymizovani pid chysla
//! kryptohrafichnoho masshtabu, yakym tsia mova ne znakhodyt zastosuvannia.
//! Eine kleine, von Hand geschriebene Ganzzahl beliebiger Genauigkeit mit
//! Vorzeichen, nur von `Rational` verwendet (siehe `value.rs`). Bewusst
//! keine Crate-Abhängigkeit: dieses Crate bleibt bei null Abhängigkeiten
//! (siehe den capability-freien Vertrag in docs/language-core.md), und
//! "Rust macht den Low-Level-Zahlenalgorithmus, in dem es besonders stark
//! ist" erfordert nicht, eine Drittanbieter-Implementierung davon
//! einzubinden — Schulbuch-Algorithmen, einfach und korrekt gehalten,
//! nicht für kryptografisch große Zahlen optimiert, für die diese Sprache
//! keine Verwendung hat.

use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

/// Unsigned magnitude: little-endian base-2^32 limbs, no trailing zero limbs
/// (an empty vec is the canonical representation of zero).
/// Bezznakova velychyna: little-endian limby bazy 2^32, bez kintsevykh
/// nulovykh limbiv (porozhnii vec — kanonichne predstavlennia nulia).
/// Vorzeichenlose Größe: Little-Endian-Limbs zur Basis 2^32, keine
/// abschließenden Null-Limbs (ein leerer Vec ist die kanonische
/// Darstellung von Null).
#[derive(Clone, Debug, PartialEq, Eq)]
struct Magnitude(Vec<u32>);

impl Magnitude {
    const ZERO: Magnitude = Magnitude(Vec::new());
    const KARATSUBA_THRESHOLD: usize = 32;

    fn from_u64(mut value: u64) -> Self {
        let mut limbs = Vec::new();
        while value != 0 {
            limbs.push((value & 0xFFFF_FFFF) as u32);
            value >>= 32;
        }
        Magnitude(limbs)
    }

    /// Number of trailing zero bits across all limbs (0 for zero magnitude).
    /// Kilkist kintsevykh nulovykh bitiv usikh limbiv.
    fn trailing_zeros(&self) -> u32 {
        let mut zeros = 0u32;
        for limb in &self.0 {
            if *limb == 0 {
                zeros += 32;
            } else {
                zeros += limb.trailing_zeros();
                break;
            }
        }
        zeros
    }

    /// In-place logical right shift by `bits` (< total bit length).
    /// Pobitovyi zsun vpravo na `bits` bil in-place.
    fn shr_assign(&mut self, bits: u32) {
        let limbs = (bits / 32) as usize;
        let rem = bits % 32;
        if limbs >= self.0.len() {
            self.0.clear();
            return;
        }
        if limbs > 0 {
            self.0.drain(0..limbs);
        }
        if rem > 0 {
            let mut carry = 0u32;
            for limb in self.0.iter_mut().rev() {
                let next_carry = *limb << (32 - rem);
                *limb = (*limb >> rem) | carry;
                carry = next_carry;
            }
        }
        while self.0.last() == Some(&0) {
            self.0.pop();
        }
    }

    /// In-place logical left shift by `bits`.
    /// Pobitovyi zsun vlivo na `bits` bil.
    fn shl_assign(&mut self, bits: u32) {
        if self.is_zero() {
            return;
        }
        let limbs = (bits / 32) as usize;
        let rem = bits % 32;
        if rem > 0 {
            let mut carry = 0u32;
            for limb in self.0.iter_mut() {
                let next_carry = *limb >> (32 - rem);
                *limb = (*limb << rem) | carry;
                carry = next_carry;
            }
            if carry != 0 {
                self.0.push(carry);
            }
        }
        if limbs > 0 {
            self.0.splice(0..0, std::iter::repeat_n(0, limbs));
        }
    }

    fn is_zero(&self) -> bool {
        self.0.is_empty()
    }

    fn trim(mut self) -> Self {
        while self.0.last() == Some(&0) {
            self.0.pop();
        }
        self
    }

    fn cmp(&self, other: &Self) -> Ordering {
        self.0
            .len()
            .cmp(&other.0.len())
            .then_with(|| self.0.iter().rev().cmp(other.0.iter().rev()))
    }

    fn add(&self, other: &Self) -> Self {
        let mut result = Vec::with_capacity(self.0.len().max(other.0.len()) + 1);
        let mut carry = 0u64;
        for i in 0..self.0.len().max(other.0.len()) {
            let sum = carry
                + u64::from(self.0.get(i).copied().unwrap_or(0))
                + u64::from(other.0.get(i).copied().unwrap_or(0));
            result.push((sum & 0xFFFF_FFFF) as u32);
            carry = sum >> 32;
        }
        if carry != 0 {
            result.push(carry as u32);
        }
        Magnitude(result).trim()
    }

    /// Assumes `self >= other`; panics otherwise (only called that way from `BigInt`).
    fn sub(&self, other: &Self) -> Self {
        let mut result = Vec::with_capacity(self.0.len());
        let mut borrow = 0i64;
        for i in 0..self.0.len() {
            let diff =
                i64::from(self.0[i]) - i64::from(other.0.get(i).copied().unwrap_or(0)) - borrow;
            if diff < 0 {
                result.push((diff + (1i64 << 32)) as u32);
                borrow = 1;
            } else {
                result.push(diff as u32);
                borrow = 0;
            }
        }
        debug_assert_eq!(borrow, 0, "sub called with self < other");
        Magnitude(result).trim()
    }

    fn mul_schoolbook(&self, other: &Self) -> Self {
        if self.is_zero() || other.is_zero() {
            return Magnitude::ZERO;
        }
        let mut result = vec![0u32; self.0.len() + other.0.len()];
        for (i, &a) in self.0.iter().enumerate() {
            let mut carry = 0u64;
            for (j, &b) in other.0.iter().enumerate() {
                let product = u64::from(a) * u64::from(b) + u64::from(result[i + j]) + carry;
                result[i + j] = (product & 0xFFFF_FFFF) as u32;
                carry = product >> 32;
            }
            let mut k = i + other.0.len();
            while carry != 0 {
                let sum = u64::from(result[k]) + carry;
                result[k] = (sum & 0xFFFF_FFFF) as u32;
                carry = sum >> 32;
                k += 1;
            }
        }
        Magnitude(result).trim()
    }

    fn shl_limbs(&self, limbs: usize) -> Self {
        if self.is_zero() {
            return Magnitude::ZERO;
        }
        let mut shifted = Vec::with_capacity(self.0.len() + limbs);
        shifted.resize(limbs, 0);
        shifted.extend_from_slice(&self.0);
        Magnitude(shifted)
    }

    /// Karatsuba multiplication for large magnitudes; schoolbook remains the
    /// base case. The split uses the smaller operand so unbalanced products
    /// never index past a short limb vector.
    fn mul(&self, other: &Self) -> Self {
        if self.is_zero() || other.is_zero() {
            return Magnitude::ZERO;
        }
        if self.0.len().min(other.0.len()) < Self::KARATSUBA_THRESHOLD {
            return self.mul_schoolbook(other);
        }
        let split = self.0.len().min(other.0.len()) / 2;
        let (a_low, a_high) = self.0.split_at(split);
        let (b_low, b_high) = other.0.split_at(split);
        let a_low = Magnitude(a_low.to_vec());
        let a_high = Magnitude(a_high.to_vec());
        let b_low = Magnitude(b_low.to_vec());
        let b_high = Magnitude(b_high.to_vec());
        let z0 = a_low.mul(&b_low);
        let z2 = a_high.mul(&b_high);
        let z1 = a_low
            .add(&a_high)
            .mul(&b_low.add(&b_high))
            .sub(&z0)
            .sub(&z2);
        z2.shl_limbs(split * 2).add(&z1.shl_limbs(split)).add(&z0)
    }

    fn bit_len(&self) -> u32 {
        match self.0.last() {
            None => 0,
            Some(&top) => (self.0.len() as u32 - 1) * 32 + (32 - top.leading_zeros()),
        }
    }

    fn get_bit(&self, index: u32) -> bool {
        let limb = (index / 32) as usize;
        let bit = index % 32;
        self.0
            .get(limb)
            .is_some_and(|&value| (value >> bit) & 1 == 1)
    }

    fn shl1(&self) -> Self {
        let mut result = Vec::with_capacity(self.0.len() + 1);
        let mut carry = 0u32;
        for &limb in &self.0 {
            result.push((limb << 1) | carry);
            carry = limb >> 31;
        }
        if carry != 0 {
            result.push(carry);
        }
        Magnitude(result).trim()
    }

    fn set_bit(&mut self, index: u32) {
        let limb = (index / 32) as usize;
        let bit = index % 32;
        if self.0.len() <= limb {
            self.0.resize(limb + 1, 0);
        }
        self.0[limb] |= 1 << bit;
    }

    /// Schoolbook binary long division — O(bit_len^2), not the fastest
    /// algorithm that exists, but correct regardless of quotient size (unlike
    /// repeated subtraction) and simple enough to read and trust. This
    /// language has no performance requirement that would justify anything
    /// more sophisticated (see the header comment).
    /// Shkilne binarne dovhe dilennia — O(bit_len^2), ne naishvydshyi
    /// alhorytm, shcho isnuie, ale korektnyi nezalezhno vid rozmiru chastky (na
    /// vidminu vid povtornoho vidnimannia) i dostatno prostyi, shchob chytaty
    /// y doviriaty. Tsia mova ne maie vymohy shvydkodii, yaka vypravdala b shchos
    /// skladnishe (dyv. header-komentar).
    /// Schulbuch-Binärlangdivision — O(bit_len^2), nicht der schnellste
    /// existierende Algorithmus, aber korrekt unabhängig von der Größe des
    /// Quotienten (anders als wiederholte Subtraktion) und einfach genug,
    /// um sie zu lesen und ihr zu vertrauen. Diese Sprache hat keine
    /// Performance-Anforderung, die etwas Ausgefeilteres rechtfertigen
    /// würde (siehe Header-Kommentar).
    fn divmod(&self, divisor: &Self) -> Option<(Self, Self)> {
        if divisor.is_zero() {
            return None;
        }
        if self.cmp(divisor) == Ordering::Less {
            return Some((Magnitude::ZERO, self.clone()));
        }
        let mut quotient = Magnitude::ZERO;
        let mut remainder = Magnitude::ZERO;
        for i in (0..self.bit_len()).rev() {
            remainder = remainder.shl1();
            if self.get_bit(i) {
                remainder = remainder.add(&Magnitude::from_u64(1));
            }
            if remainder.cmp(divisor) != Ordering::Less {
                remainder = remainder.sub(divisor);
                quotient.set_bit(i);
            }
        }
        Some((quotient.trim(), remainder))
    }

    fn to_f64(&self) -> f64 {
        self.0
            .iter()
            .rev()
            .fold(0.0, |acc, &limb| acc * 4294967296.0 + f64::from(limb))
    }

    fn to_u64(&self) -> Option<u64> {
        match self.0.len() {
            0 => Some(0),
            1 => Some(u64::from(self.0[0])),
            2 => Some(u64::from(self.0[0]) | (u64::from(self.0[1]) << 32)),
            _ => None,
        }
    }
}

impl fmt::Display for Magnitude {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_zero() {
            return write!(formatter, "0");
        }
        // Repeatedly divide by 10^9 so each step peels off up to nine decimal
        // digits at once instead of one, keeping this from being needlessly
        // slow for large numbers despite the O(n^2) division underneath.
        let mut chunks = Vec::new();
        let mut remaining = self.clone();
        let ten_to_nine = Magnitude::from_u64(1_000_000_000);
        while !remaining.is_zero() {
            let (quotient, remainder) = remaining.divmod(&ten_to_nine).expect("divisor is nonzero");
            chunks.push(remainder.to_u64().expect("< 10^9 fits in u64") as u32);
            remaining = quotient;
        }
        write!(formatter, "{}", chunks.pop().unwrap_or(0))?;
        for chunk in chunks.into_iter().rev() {
            write!(formatter, "{chunk:09}")?;
        }
        Ok(())
    }
}

/// An arbitrary-precision signed integer: sign plus magnitude. Zero is
/// always represented with `negative: false` so that `PartialEq`/`Eq` (both
/// derived) and `Magnitude`'s empty-vec zero stay a single canonical form —
/// there's no separate "negative zero" this type could accidentally produce.
/// Tsile chyslo dovilnoi tochnosti zi znakom: znak plius velychyna. Nul
/// zavzhdy predstavlenyi z `negative: false`, tozh `PartialEq`/`Eq` (obydva
/// derived) i porozhnii-vec nul `Magnitude` lyshaiutsia yedynoiu kanonichnoiu
/// formoiu — nemaie okremoho "vidiemnoho nulia", yakyi tsei typ mih by
/// vypadkovo vyrobyty.
/// Eine Ganzzahl beliebiger Genauigkeit mit Vorzeichen: Vorzeichen plus
/// Größe. Null wird immer mit `negative: false` dargestellt, sodass
/// `PartialEq`/`Eq` (beide derived) und `Magnitude`s leerer-Vec-Null eine
/// einzige kanonische Form bleiben — es gibt keine separate "negative
/// Null", die dieser Typ versehentlich erzeugen könnte.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BigInt {
    negative: bool,
    magnitude: Magnitude,
}

impl BigInt {
    pub fn from_i64(value: i64) -> Self {
        BigInt {
            negative: value < 0,
            magnitude: Magnitude::from_u64(value.unsigned_abs()),
        }
    }

    pub fn zero() -> Self {
        BigInt {
            negative: false,
            magnitude: Magnitude::ZERO,
        }
    }

    /// Parses an explicit base-2 integer used by the canonical numeric wire.
    /// This is not the ordinary source-number reader: the caller must already
    /// have selected the tagged wire grammar.
    pub(crate) fn from_binary_str(text: &str) -> Result<Self, ()> {
        let (negative, digits) = match text.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, text.strip_prefix('+').unwrap_or(text)),
        };
        if digits.is_empty() || !digits.bytes().all(|byte| matches!(byte, b'0' | b'1')) {
            return Err(());
        }

        let one = Magnitude::from_u64(1);
        let mut magnitude = Magnitude::ZERO;
        for byte in digits.bytes() {
            magnitude.shl_assign(1);
            if byte == b'1' {
                magnitude = magnitude.add(&one);
            }
        }
        Ok(Self::normalized(negative, magnitude))
    }

    /// Canonical base-2 spelling for the tagged numeric wire. Magnitude uses
    /// no leading zeros; zero is always exactly "0".
    pub(crate) fn to_binary_string(&self) -> String {
        if self.is_zero() {
            return "0".to_string();
        }
        let width = self.bit_length();
        let mut out = String::with_capacity(width + self.negative as usize);
        if self.negative {
            out.push('-');
        }
        for bit in (0..width).rev() {
            let limb = self.magnitude.0[bit / 32];
            let value = (limb >> (bit % 32)) & 1;
            out.push(if value == 0 { '0' } else { '1' });
        }
        out
    }

    pub fn is_zero(&self) -> bool {
        self.magnitude.is_zero()
    }

    pub fn is_negative(&self) -> bool {
        self.negative
    }

    /// Bit width of the magnitude (sign ignored), `0` for zero itself —
    /// used only to enforce an *opt-in* numeric resource limit
    /// (`Environment::with_numeric_bit_limit`), never in ordinary
    /// arithmetic. The reference Rust implementation stays unbounded by
    /// default (see `docs/language-core-axioms.md`'s S1).
    /// Shyryna velychyny v bitakh (znak ihnoruietsia), `0` dlia samoho nulia —
    /// vykorystovuietsia lyshe dlia prymusovoho *optsiinoho* chyslovoho
    /// obmezhennia resursu (`Environment::with_numeric_bit_limit`), nikoly v
    /// zvychainii aryfmetytsi. Etalonna Rust-realizatsiia lyshaietsia
    /// neobmezhenoiu za zamovchuvanniam (dyv. S1 u `docs/language-core-axioms.md`).
    /// Remainder of self divided by a single-limb magnitude divisor.
    /// O(limbs) linear pass — used by the gcd small-operand fast path.
    fn rem_by_limb(&self, divisor: u32) -> u32 {
        let mut rem: u64 = 0;
        for limb in self.magnitude.0.iter().rev() {
            rem = ((rem << 32) | (*limb as u64)) % (divisor as u64);
        }
        rem as u32
    }

    /// FASL support: append sign + little-endian limb encoding to `out`.
    /// Pidtrymka FASL: znak + LE limby u `out`.
    pub(crate) fn write_fasl(&self, out: &mut Vec<u8>) {
        out.push(self.is_negative() as u8);
        out.extend_from_slice(&(self.magnitude.0.len() as u32).to_le_bytes());
        for limb in &self.magnitude.0 {
            out.extend_from_slice(&limb.to_le_bytes());
        }
    }

    /// FASL inverse: parse what `write_fasl` produced. Returns None on
    /// truncation/corruption so callers fall back to text parsing.
    pub(crate) fn read_fasl(bytes: &[u8], pos: &mut usize) -> Option<Self> {
        let tag = *bytes.get(*pos)?;
        *pos += 1;
        let len = u32::from_le_bytes(bytes.get(*pos..*pos + 4)?.try_into().ok()?) as usize;
        *pos += 4;
        let mut limbs = Vec::with_capacity(len.min(1 << 20));
        for _ in 0..len {
            let limb = u32::from_le_bytes(bytes.get(*pos..*pos + 4)?.try_into().ok()?);
            *pos += 4;
            limbs.push(limb);
        }
        if tag > 1 {
            return None;
        }
        Some(Self::normalized(tag == 1, Magnitude(limbs)))
    }

    pub fn bit_length(&self) -> usize {
        match self.magnitude.0.last() {
            None => 0,
            Some(&top) => (self.magnitude.0.len() - 1) * 32 + (32 - top.leading_zeros() as usize),
        }
    }

    fn normalized(negative: bool, magnitude: Magnitude) -> Self {
        let is_zero = magnitude.is_zero();
        BigInt {
            negative: negative && !is_zero,
            magnitude,
        }
    }

    pub fn neg(&self) -> Self {
        Self::normalized(!self.negative, self.magnitude.clone())
    }

    pub fn abs(&self) -> Self {
        Self::normalized(false, self.magnitude.clone())
    }

    pub fn add(&self, other: &Self) -> Self {
        if self.negative == other.negative {
            Self::normalized(self.negative, self.magnitude.add(&other.magnitude))
        } else {
            match self.magnitude.cmp(&other.magnitude) {
                Ordering::Equal => Self::zero(),
                Ordering::Greater => {
                    Self::normalized(self.negative, self.magnitude.sub(&other.magnitude))
                }
                Ordering::Less => {
                    Self::normalized(other.negative, other.magnitude.sub(&self.magnitude))
                }
            }
        }
    }

    pub fn sub(&self, other: &Self) -> Self {
        self.add(&other.neg())
    }

    pub fn mul(&self, other: &Self) -> Self {
        Self::normalized(
            self.negative != other.negative,
            self.magnitude.mul(&other.magnitude),
        )
    }

    /// Truncating division (remainder takes the dividend's sign, same as
    /// Rust's own `//%` on integers) — the only caller, `Rational`'s GCD
    /// reduction and exact-division-by-gcd, always works with either
    /// non-negative operands or a remainder it discards, so the rounding
    /// direction on negative operands is never actually observed.
    /// Dilennia z vidkydanniam (zalyshok bere znak dilenoho, tak samo yak
    /// vbudovani `/`/`%` Rust dlia tsilykh) — yedynyi vyklyk, skorochennia GCD i
    /// tochne dilennia na gcd u `Rational`, zavzhdy pratsiuie abo z nevidiemnymy
    /// operandamy, abo z zalyshkom, yakyi vidkydaietsia, tozh napriam
    /// okruhlennia dlia vidiemnykh operandiv naspravdi nikoly ne
    /// sposterihaietsia.
    /// Abschneidende Division (Rest übernimmt das Vorzeichen des
    /// Dividenden, genau wie Rusts eigene `/`/`%` bei Ganzzahlen) — der
    /// einzige Aufrufer, die GGT-Reduktion und exakte Division durch den
    /// GGT in `Rational`, arbeitet immer entweder mit nichtnegativen
    /// Operanden oder einem verworfenen Rest, daher wird die
    /// Rundungsrichtung bei negativen Operanden nie tatsächlich
    /// beobachtet.
    pub fn div_rem(&self, other: &Self) -> Option<(Self, Self)> {
        let (quotient, remainder) = self.magnitude.divmod(&other.magnitude)?;
        Some((
            Self::normalized(self.negative != other.negative, quotient),
            Self::normalized(self.negative, remainder),
        ))
    }

    pub fn cmp(&self, other: &Self) -> Ordering {
        match (self.negative, other.negative) {
            (false, true) => Ordering::Greater,
            (true, false) => Ordering::Less,
            (false, false) => self.magnitude.cmp(&other.magnitude),
            (true, true) => other.magnitude.cmp(&self.magnitude),
        }
    }

    pub fn to_f64(&self) -> f64 {
        let value = self.magnitude.to_f64();
        if self.negative {
            -value
        } else {
            value
        }
    }

    pub fn to_i64(&self) -> Option<i64> {
        let magnitude = self.magnitude.to_u64()?;
        if self.negative {
            (magnitude <= i64::MAX as u64 + 1).then(|| {
                if magnitude == i64::MAX as u64 + 1 {
                    i64::MIN
                } else {
                    -(magnitude as i64)
                }
            })
        } else {
            i64::try_from(magnitude).ok()
        }
    }

    /// Euclidean algorithm via `div_rem`; both inputs are used through
    /// their absolute value, so the result is always non-negative,
    /// matching the convention `Rational`'s reduction step expects.
    /// Alhorytm Evklida cherez `div_rem`; obydva vkhody vykorystovuiutsia
    /// cherez svii modul, tozh rezultat zavzhdy nevidiemnyi, vidpovidno do
    /// konventsii, yakoi ochikuie krok skorochennia `Rational`.
    /// Euklidischer Algorithmus über `div_rem`; beide Eingaben werden über
    /// ihren Absolutwert verwendet, daher ist das Ergebnis immer
    /// nichtnegativ, passend zur Konvention, die der Reduktionsschritt von
    /// `Rational` erwartet.
    pub fn gcd(&self, other: &Self) -> Self {
        // Fast path: huge × small (≤64-bit) — the rational-chain shape is
        // exactly gcd(giant accumulated denominator, small term denominator).
        // One linear pass computes giant % small, and from there both
        // operands are tiny; full binary stepping on equal-size giants
        // would be orders of magnitude slower.
        if other.magnitude.0.len() == 1
            && !other.magnitude.0.is_empty()
            && self.magnitude.0.len() > 2
        {
            let divisor = other.magnitude.0[0];
            let r = self.rem_by_limb(divisor);
            let mut x = BigInt::from_i64(r as i64);
            let mut y = BigInt::from_i64(divisor as i64);
            while !y.is_zero() {
                let (_, rem) = x.div_rem(&y).expect("nonzero divisor");
                x = y;
                y = rem;
            }
            return x.abs();
        }
        // Stein's binary GCD: shifts and subtractions only — no div_rem.
        // Each Rational reduction step pays this cost, and div_rem-based
        // Euclid was the dominant term in exact-rational chains
        // (benchmarks/rational stress, WSM-24 shape).
        // Binarnyi gcd Shtaina: tilky zsuny ta vidnimannia — bez div_rem;
        // same skorochennia Rational platytse za tsei krok.
        if self.is_zero() {
            return other.abs();
        }
        if other.is_zero() {
            return self.abs();
        }
        let a_mag = self.abs().magnitude;
        let b_mag = other.abs().magnitude;
        let shared = a_mag.trailing_zeros().min(b_mag.trailing_zeros());
        let mut a = a_mag;
        a.shr_assign(a.trailing_zeros());
        let mut b = b_mag;
        loop {
            b.shr_assign(b.trailing_zeros());
            if a.cmp(&b) == Ordering::Greater {
                std::mem::swap(&mut a, &mut b);
            }
            b = b.sub(&a);
            if b.is_zero() {
                break;
            }
        }
        a.shl_assign(shared);
        BigInt::normalized(false, a)
    }
}

impl FromStr for BigInt {
    type Err = ();

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (negative, digits) = match text.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, text.strip_prefix('+').unwrap_or(text)),
        };
        if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return Err(());
        }
        let mut magnitude = Magnitude::ZERO;
        let ten = Magnitude::from_u64(10);
        for byte in digits.bytes() {
            magnitude = magnitude
                .mul(&ten)
                .add(&Magnitude::from_u64(u64::from(byte - b'0')));
        }
        Ok(BigInt::normalized(negative, magnitude))
    }
}

impl fmt::Display for BigInt {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.negative {
            write!(formatter, "-")?;
        }
        write!(formatter, "{}", self.magnitude)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic differential corpus: Stein (production gcd) must agree
    /// with the retired Euclid-divmod algorithm on every sample — values,
    /// signs, and canonical zero. Runtime is microseconds-scale by design
    /// (owner resource policy: targeted evidence, not a heavy gate).
    fn euclid_gcd_reference(a: &BigInt, b: &BigInt) -> BigInt {
        let mut a = a.abs();
        let mut b = b.abs();
        while !b.is_zero() {
            let (_, remainder) = a.div_rem(&b).expect("nonzero divisor");
            a = b;
            b = remainder;
        }
        a
    }

    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            let mut x = self.0;
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            self.0 = x;
            x.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }
        fn limb_count(&mut self) -> usize {
            (self.next() % 9) as usize // 0..=8 limbs => up to 256-bit
        }
        fn magnitude(&mut self) -> Magnitude {
            let count = self.limb_count();
            let mut limbs: Vec<u32> = (0..count)
                .map(|_| (self.next() & 0xFFFF_FFFF) as u32)
                .collect();
            while let Some(&0) = limbs.last() {
                limbs.pop();
            }
            Magnitude(limbs)
        }
        fn bigint(&mut self) -> BigInt {
            let negative = self.next() & 1 == 0 && !self.magnitude().is_zero();
            let magnitude = self.magnitude();
            BigInt::normalized(negative && !magnitude.is_zero(), magnitude)
        }
    }

    #[test]
    fn binary_gcd_matches_euclid_on_deterministic_corpus() {
        let mut rng = Rng(0x5EED_2026_0824_00C1);
        for _ in 0..600 {
            let a = rng.bigint();
            let b = rng.bigint();
            assert_eq!(
                a.gcd(&b),
                euclid_gcd_reference(&a, &b),
                "stein vs euclid diverged on a={a} b={b}"
            );
        }
    }

    #[test]
    fn binary_gcd_handles_power_of_two_and_shared_factor_shapes() {
        // shared power-of-two factor must come back via the final SHL
        let two_pow_40 = BigInt::normalized(false, {
            let mut m = Magnitude::from_u64(1); // 1 << 40 — not the zero magnitude
            m.shl_assign(40);
            m
        });
        let twelve = BigInt::from_i64(12);
        assert_eq!(two_pow_40.gcd(&twelve).to_i64(), Some(4));
        assert_eq!(BigInt::from_i64(0).gcd(&BigInt::zero()).to_i64(), Some(0));
    }

    #[test]
    fn add_sub_mul_match_i64_for_small_values() {
        let a = BigInt::from_i64(123_456_789);
        let b = BigInt::from_i64(-987_654_321);
        assert_eq!(a.add(&b).to_i64(), Some(123_456_789 - 987_654_321));
        assert_eq!(a.sub(&b).to_i64(), Some(123_456_789 + 987_654_321));
        assert_eq!(a.mul(&b).to_i64(), Some(123_456_789i64 * -987_654_321));
    }

    #[test]
    fn multiplication_exceeds_i64_without_overflowing() {
        let big = BigInt::from_i64(i64::MAX).mul(&BigInt::from_i64(i64::MAX));
        assert_eq!(big.to_i64(), None);
        assert_eq!(big.to_string(), "85070591730234615847396907784232501249");
    }

    #[test]
    fn karatsuba_matches_schoolbook_at_and_above_threshold() {
        let mut state = 0x9E37_79B9u64;
        let mut next_limb = || {
            state ^= state << 7;
            state ^= state >> 9;
            state ^= state << 8;
            state as u32
        };
        for (left_len, right_len) in [(31, 31), (32, 32), (33, 65), (64, 32), (70, 71)] {
            let left = Magnitude((0..left_len).map(|_| next_limb()).collect()).trim();
            let right = Magnitude((0..right_len).map(|_| next_limb()).collect()).trim();
            assert_eq!(left.mul(&right), left.mul_schoolbook(&right));
        }
        let max_limb = Magnitude(vec![u32::MAX; 65]);
        assert_eq!(max_limb.mul(&max_limb), max_limb.mul_schoolbook(&max_limb));
    }

    #[test]
    fn karatsuba_schoolbook_same_build_ab_probe() {
        use std::time::Instant;

        let left = Magnitude(
            (0..256)
                .map(|i| (i as u32).wrapping_mul(0x9E37_79B9))
                .collect(),
        )
        .trim();
        let right = Magnitude(
            (0..256)
                .map(|i| (i as u32).wrapping_mul(0x85EB_CA6B).rotate_left(7))
                .collect(),
        )
        .trim();
        let repetitions = 20;
        let start_schoolbook = Instant::now();
        let mut schoolbook = Magnitude::ZERO;
        for _ in 0..repetitions {
            schoolbook = left.mul_schoolbook(&right);
        }
        let schoolbook_ns = start_schoolbook.elapsed().as_nanos();
        let start_karatsuba = Instant::now();
        let mut karatsuba = Magnitude::ZERO;
        for _ in 0..repetitions {
            karatsuba = left.mul(&right);
        }
        let karatsuba_ns = start_karatsuba.elapsed().as_nanos();
        assert_eq!(karatsuba, schoolbook);
        println!(
            "KARATSUBA_AB limbs=256 repetitions={repetitions} schoolbook_ns={schoolbook_ns} karatsuba_ns={karatsuba_ns}"
        );
    }

    #[test]
    fn division_and_remainder_match_i64_semantics() {
        let a = BigInt::from_i64(1_000_000_007);
        let b = BigInt::from_i64(97);
        let (q, r) = a.div_rem(&b).unwrap();
        assert_eq!(q.to_i64(), Some(1_000_000_007 / 97));
        assert_eq!(r.to_i64(), Some(1_000_000_007 % 97));
    }

    #[test]
    fn division_by_zero_returns_none() {
        assert!(BigInt::from_i64(5).div_rem(&BigInt::zero()).is_none());
    }

    #[test]
    fn gcd_matches_known_values() {
        assert_eq!(
            BigInt::from_i64(48).gcd(&BigInt::from_i64(18)).to_i64(),
            Some(6)
        );
        assert_eq!(
            BigInt::from_i64(0).gcd(&BigInt::from_i64(5)).to_i64(),
            Some(5)
        );
    }

    #[test]
    fn from_str_round_trips_through_display() {
        let huge = "123456789012345678901234567890";
        assert_eq!(BigInt::from_str(huge).unwrap().to_string(), huge);
        let negative = "-42";
        assert_eq!(BigInt::from_str(negative).unwrap().to_string(), negative);
        assert!(BigInt::from_str("").is_err());
        assert!(BigInt::from_str("12a").is_err());
    }

    #[test]
    fn zero_is_never_represented_as_negative() {
        let a = BigInt::from_i64(5);
        let b = BigInt::from_i64(5);
        assert_eq!(a.sub(&b), BigInt::zero());
        assert!(!a.sub(&b).is_negative());
    }

    #[test]
    fn ordering_accounts_for_sign_and_magnitude() {
        assert_eq!(
            BigInt::from_i64(-5).cmp(&BigInt::from_i64(3)),
            Ordering::Less
        );
        assert_eq!(
            BigInt::from_i64(5).cmp(&BigInt::from_i64(3)),
            Ordering::Greater
        );
        assert_eq!(
            BigInt::from_i64(-5).cmp(&BigInt::from_i64(-3)),
            Ordering::Less
        );
    }
    #[test]
    fn binary_wire_helpers_are_canonical_and_arbitrary_precision() {
        for (input, expected) in [
            ("0", "0"),
            ("0000", "0"),
            ("101010", "101010"),
            ("-00101", "-101"),
        ] {
            assert_eq!(
                BigInt::from_binary_str(input).unwrap().to_binary_string(),
                expected
            );
        }

        let huge = format!("1{}", "0".repeat(192));
        assert_eq!(
            BigInt::from_binary_str(&huge).unwrap().to_binary_string(),
            huge
        );
        assert!(BigInt::from_binary_str("").is_err());
        assert!(BigInt::from_binary_str("102").is_err());
    }

}
