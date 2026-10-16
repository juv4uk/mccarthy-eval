//! Research-only F3M-5λ candidate: closed de Bruijn lambda terms in five bytes.
//!
//! This does not replace the already-proved F3M-5 mapping.  It explores a
//! stronger grammar where one-argument LAMBDA has no textual parameter name.
//! Bound variables are de Bruijn references: BOUND(0) is the nearest binder.
//! Thus λx.x, λy.y, and λfoo.foo share one semantic identity: λ.BOUND(0).
//!
//! The candidate keeps the frozen F3M-1..4 grammar as state 0 and enumerates
//! only extension terms as state 1, so no 1..4-byte program is duplicated in
//! this 40-bit research space.

use crate::frame3_micro_enumerator::{
    Frame3MicroEnumerateError, CAP_ATOM, CAP_CAR, CAP_CDR, CAP_COND, CAP_CONS,
    CAP_EQ, CAP_LAMBDA, CAP_QUOTE,
};
use crate::{
    Bija3, Bit1, Bit3, Bit4, CoreD4, CoreDomainIdentity, DomainIdentity, ErrorKind, Expr,
    ExprKind, LanguageError, PredicateBit, Session, Span, Value,
};
use std::{rc::Rc, sync::OnceLock};

pub const F3M5_LAMBDA_CAPACITY: u128 = 1u128 << 40;
const MAX_NODES: usize = 13;
const MAX_DEPTH: usize = MAX_NODES + 1;
type Counts = [[[u128; 2]; MAX_DEPTH + 1]; MAX_NODES + 1];

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum F3m5LambdaExpr {
    Zero,
    One,
    Nil,
    QuoteZero,
    QuoteOne,
    QuoteNil,
    Bound(u8),
    Atom(Box<Self>),
    Cons(Box<Self>, Box<Self>),
    CarCons(Box<Self>, Box<Self>),
    CdrCons(Box<Self>, Box<Self>),
    EqAtom(Box<Self>, Box<Self>),
    Lambda(Box<Self>),
    IfAtom(Box<Self>, Box<Self>, Box<Self>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum F3m5LambdaValue {
    Bit(bool),
    Nil,
    Pair(Box<Self>, Box<Self>),
    Closure {
        body: Box<F3m5LambdaExpr>,
        env: Vec<F3m5LambdaValue>,
    },
}

fn add(a: u128, b: u128) -> Result<u128, Frame3MicroEnumerateError> {
    a.checked_add(b)
        .ok_or(Frame3MicroEnumerateError::InternalCountOverflow)
}
fn mul(a: u128, b: u128) -> Result<u128, Frame3MicroEnumerateError> {
    a.checked_mul(b)
        .ok_or(Frame3MicroEnumerateError::InternalCountOverflow)
}
fn total(c: &Counts, n: usize, d: usize) -> Result<u128, Frame3MicroEnumerateError> {
    add(c[n][d][0], c[n][d][1])
}

fn build_counts() -> Result<Counts, Frame3MicroEnumerateError> {
    let mut c = [[[0u128; 2]; MAX_DEPTH + 1]; MAX_NODES + 1];
    for d in 0..=MAX_DEPTH {
        c[1][d] = [6, d as u128];
    }

    for n in 2..=MAX_NODES {
        for d in (0..MAX_DEPTH).rev() {
            let mut old = c[n - 1][d][0];

            let mut old_pairs = 0;
            for l in 1..n - 1 {
                let r = n - 1 - l;
                old_pairs = add(old_pairs, mul(c[l][d][0], c[r][d][0])?)?;
            }
            old = add(old, mul(3, old_pairs)?)?;

            for t in 1..n.saturating_sub(2) {
                for y in 1..n - 1 - t {
                    let z = n - 1 - t - y;
                    if z == 0 {
                        continue;
                    }
                    old = add(old, mul(mul(c[t][d][0], c[y][d][0])?, c[z][d][0])?)?;
                }
            }

            // ATOM(new child) + LAMBDA(any child one binder deeper).
            let mut new = add(c[n - 1][d][1], total(&c, n - 1, d + 1)?)?;

            // CONS/CAR/CDR with >=1 new child, plus EQ with any children.
            for l in 1..n - 1 {
                let r = n - 1 - l;
                let all = mul(total(&c, l, d)?, total(&c, r, d)?)?;
                let old_pair = mul(c[l][d][0], c[r][d][0])?;
                new = add(new, mul(3, all - old_pair)?)?;
                new = add(new, all)?;
            }

            // IF-ATOM with >=1 new child.
            for t in 1..n.saturating_sub(2) {
                for y in 1..n - 1 - t {
                    let z = n - 1 - t - y;
                    if z == 0 {
                        continue;
                    }
                    let all = mul(mul(total(&c, t, d)?, total(&c, y, d)?)?, total(&c, z, d)?)?;
                    let old_triple = mul(mul(c[t][d][0], c[y][d][0])?, c[z][d][0])?;
                    new = add(new, all - old_triple)?;
                }
            }

            c[n][d] = [old, new];
        }
    }
    Ok(c)
}

fn counts() -> &'static Counts {
    static C: OnceLock<Counts> = OnceLock::new();
    C.get_or_init(|| build_counts().expect("bounded F3M-5λ counts fit u128"))
}

impl F3m5LambdaExpr {
    pub fn node_count(&self) -> usize {
        match self {
            Self::Zero
            | Self::One
            | Self::Nil
            | Self::QuoteZero
            | Self::QuoteOne
            | Self::QuoteNil
            | Self::Bound(_) => 1,
            Self::Atom(x) | Self::Lambda(x) => 1 + x.node_count(),
            Self::Cons(a, b) | Self::CarCons(a, b) | Self::CdrCons(a, b) | Self::EqAtom(a, b) => {
                1 + a.node_count() + b.node_count()
            }
            Self::IfAtom(a, b, c) => 1 + a.node_count() + b.node_count() + c.node_count(),
        }
    }

    pub fn contains_extension(&self) -> bool {
        match self {
            Self::Bound(_) | Self::Lambda(_) | Self::EqAtom(_, _) => true,
            Self::Atom(x) => x.contains_extension(),
            Self::Cons(a, b) | Self::CarCons(a, b) | Self::CdrCons(a, b) => {
                a.contains_extension() || b.contains_extension()
            }
            Self::IfAtom(a, b, c) => {
                a.contains_extension() || b.contains_extension() || c.contains_extension()
            }
            _ => false,
        }
    }

    pub fn capability_mask(&self) -> u8 {
        match self {
            Self::Zero | Self::One | Self::Nil | Self::Bound(_) => 0,
            Self::QuoteZero | Self::QuoteOne | Self::QuoteNil => CAP_QUOTE,
            Self::Atom(x) => CAP_ATOM | x.capability_mask(),
            Self::Cons(a, b) => CAP_CONS | a.capability_mask() | b.capability_mask(),
            Self::CarCons(a, b) => CAP_CAR | CAP_CONS | a.capability_mask() | b.capability_mask(),
            Self::CdrCons(a, b) => CAP_CDR | CAP_CONS | a.capability_mask() | b.capability_mask(),
            Self::EqAtom(a, b) => CAP_EQ | CAP_ATOM | a.capability_mask() | b.capability_mask(),
            Self::Lambda(x) => CAP_LAMBDA | x.capability_mask(),
            Self::IfAtom(a, b, c) => {
                CAP_COND | CAP_ATOM | a.capability_mask() | b.capability_mask() | c.capability_mask()
            }
        }
    }

    pub fn human_projection(&self) -> String {
        match self {
            Self::Zero => "0".into(),
            Self::One => "1".into(),
            Self::Nil => "NIL".into(),
            Self::QuoteZero => "QUOTE(0)".into(),
            Self::QuoteOne => "QUOTE(1)".into(),
            Self::QuoteNil => "QUOTE(NIL)".into(),
            Self::Bound(i) => format!("BOUND({i})"),
            Self::Atom(x) => format!("ATOM({})", x.human_projection()),
            Self::Cons(a, b) => format!("CONS({},{})", a.human_projection(), b.human_projection()),
            Self::CarCons(a, b) => {
                format!("CAR(CONS({},{}))", a.human_projection(), b.human_projection())
            }
            Self::CdrCons(a, b) => {
                format!("CDR(CONS({},{}))", a.human_projection(), b.human_projection())
            }
            Self::EqAtom(a, b) => {
                format!("EQ(ATOM({}),ATOM({}))", a.human_projection(), b.human_projection())
            }
            Self::Lambda(x) => format!("λ.{}", x.human_projection()),
            Self::IfAtom(a, b, c) => format!(
                "IF-ATOM({},{},{})",
                a.human_projection(),
                b.human_projection(),
                c.human_projection()
            ),
        }
    }
}

fn old_leaf(rank: u128) -> Result<F3m5LambdaExpr, Frame3MicroEnumerateError> {
    match rank {
        0 => Ok(F3m5LambdaExpr::Zero),
        1 => Ok(F3m5LambdaExpr::One),
        2 => Ok(F3m5LambdaExpr::Nil),
        3 => Ok(F3m5LambdaExpr::QuoteZero),
        4 => Ok(F3m5LambdaExpr::QuoteOne),
        5 => Ok(F3m5LambdaExpr::QuoteNil),
        _ => Err(Frame3MicroEnumerateError::RankOutOfRange),
    }
}
fn old_leaf_rank(e: &F3m5LambdaExpr) -> Option<u128> {
    match e {
        F3m5LambdaExpr::Zero => Some(0),
        F3m5LambdaExpr::One => Some(1),
        F3m5LambdaExpr::Nil => Some(2),
        F3m5LambdaExpr::QuoteZero => Some(3),
        F3m5LambdaExpr::QuoteOne => Some(4),
        F3m5LambdaExpr::QuoteNil => Some(5),
        _ => None,
    }
}

fn unrank_old(
    n: usize,
    d: usize,
    mut rank: u128,
    c: &Counts,
) -> Result<F3m5LambdaExpr, Frame3MicroEnumerateError> {
    if n == 1 {
        return old_leaf(rank);
    }
    if rank >= c[n][d][0] {
        return Err(Frame3MicroEnumerateError::RankOutOfRange);
    }

    let block = c[n - 1][d][0];
    if rank < block {
        return Ok(F3m5LambdaExpr::Atom(Box::new(unrank_old(n - 1, d, rank, c)?)));
    }
    rank -= block;

    for family in 0..3 {
        for l in 1..n - 1 {
            let r = n - 1 - l;
            let rc = c[r][d][0];
            let block = mul(c[l][d][0], rc)?;
            if rank < block {
                let a = Box::new(unrank_old(l, d, rank / rc, c)?);
                let b = Box::new(unrank_old(r, d, rank % rc, c)?);
                return Ok(match family {
                    0 => F3m5LambdaExpr::Cons(a, b),
                    1 => F3m5LambdaExpr::CarCons(a, b),
                    _ => F3m5LambdaExpr::CdrCons(a, b),
                });
            }
            rank -= block;
        }
    }

    for t in 1..n.saturating_sub(2) {
        for y in 1..n - 1 - t {
            let z = n - 1 - t - y;
            if z == 0 {
                continue;
            }
            let yz = mul(c[y][d][0], c[z][d][0])?;
            let block = mul(c[t][d][0], yz)?;
            if rank < block {
                let tr = rank / yz;
                let rem = rank % yz;
                return Ok(F3m5LambdaExpr::IfAtom(
                    Box::new(unrank_old(t, d, tr, c)?),
                    Box::new(unrank_old(y, d, rem / c[z][d][0], c)?),
                    Box::new(unrank_old(z, d, rem % c[z][d][0], c)?),
                ));
            }
            rank -= block;
        }
    }
    Err(Frame3MicroEnumerateError::RankOutOfRange)
}

fn rank_old(
    e: &F3m5LambdaExpr,
    d: usize,
    c: &Counts,
) -> Result<u128, Frame3MicroEnumerateError> {
    if e.contains_extension() {
        return Err(Frame3MicroEnumerateError::RankOutOfRange);
    }
    let n = e.node_count();
    if n == 1 {
        return old_leaf_rank(e).ok_or(Frame3MicroEnumerateError::RankOutOfRange);
    }

    if let F3m5LambdaExpr::Atom(x) = e {
        return rank_old(x, d, c);
    }
    let mut rank = c[n - 1][d][0];

    let mut pairs = 0;
    for l in 1..n - 1 {
        pairs = add(pairs, mul(c[l][d][0], c[n - 1 - l][d][0])?)?;
    }

    let binary = match e {
        F3m5LambdaExpr::Cons(a, b) => Some((0u128, a, b)),
        F3m5LambdaExpr::CarCons(a, b) => Some((1u128, a, b)),
        F3m5LambdaExpr::CdrCons(a, b) => Some((2u128, a, b)),
        _ => None,
    };
    if let Some((family, a, b)) = binary {
        rank = add(rank, mul(family, pairs)?)?;
        let an = a.node_count();
        let bn = b.node_count();
        for l in 1..an {
            rank = add(rank, mul(c[l][d][0], c[n - 1 - l][d][0])?)?;
        }
        rank = add(rank, mul(rank_old(a, d, c)?, c[bn][d][0])?)?;
        return add(rank, rank_old(b, d, c)?);
    }

    rank = add(rank, mul(3, pairs)?)?;
    if let F3m5LambdaExpr::IfAtom(a, b, z) = e {
        let an = a.node_count();
        let bn = b.node_count();
        let zn = z.node_count();

        for t in 1..an {
            for y in 1..n - 1 - t {
                let zz = n - 1 - t - y;
                if zz > 0 {
                    rank = add(rank, mul(mul(c[t][d][0], c[y][d][0])?, c[zz][d][0])?)?;
                }
            }
        }
        for y in 1..bn {
            let zz = n - 1 - an - y;
            if zz > 0 {
                rank = add(rank, mul(mul(c[an][d][0], c[y][d][0])?, c[zz][d][0])?)?;
            }
        }

        let bz = mul(c[bn][d][0], c[zn][d][0])?;
        rank = add(rank, mul(rank_old(a, d, c)?, bz)?)?;
        rank = add(rank, mul(rank_old(b, d, c)?, c[zn][d][0])?)?;
        return add(rank, rank_old(z, d, c)?);
    }
    Err(Frame3MicroEnumerateError::RankOutOfRange)
}

fn unrank_new(
    n: usize,
    d: usize,
    mut rank: u128,
    c: &Counts,
) -> Result<F3m5LambdaExpr, Frame3MicroEnumerateError> {
    if rank >= c[n][d][1] {
        return Err(Frame3MicroEnumerateError::RankOutOfRange);
    }
    if n == 1 {
        return (rank < d as u128)
            .then(|| F3m5LambdaExpr::Bound(rank as u8))
            .ok_or(Frame3MicroEnumerateError::RankOutOfRange);
    }

    let block = c[n - 1][d][1];
    if rank < block {
        return Ok(F3m5LambdaExpr::Atom(Box::new(unrank_new(n - 1, d, rank, c)?)));
    }
    rank -= block;

    // New child first makes payload 0000000000 exactly λ.BOUND(0).
    for child_new in [true, false] {
        let block = c[n - 1][d + 1][usize::from(child_new)];
        if rank < block {
            let body = if child_new {
                unrank_new(n - 1, d + 1, rank, c)?
            } else {
                unrank_old(n - 1, d + 1, rank, c)?
            };
            return Ok(F3m5LambdaExpr::Lambda(Box::new(body)));
        }
        rank -= block;
    }

    for family in 0..3 {
        for l in 1..n - 1 {
            let r = n - 1 - l;
            for (ln, rn) in [(false, true), (true, false), (true, true)] {
                let rc = c[r][d][usize::from(rn)];
                let block = mul(c[l][d][usize::from(ln)], rc)?;
                if rank < block {
                    let ar = rank / rc;
                    let br = rank % rc;
                    let a = if ln { unrank_new(l, d, ar, c)? } else { unrank_old(l, d, ar, c)? };
                    let b = if rn { unrank_new(r, d, br, c)? } else { unrank_old(r, d, br, c)? };
                    return Ok(match family {
                        0 => F3m5LambdaExpr::Cons(Box::new(a), Box::new(b)),
                        1 => F3m5LambdaExpr::CarCons(Box::new(a), Box::new(b)),
                        _ => F3m5LambdaExpr::CdrCons(Box::new(a), Box::new(b)),
                    });
                }
                rank -= block;
            }
        }
    }

    for l in 1..n - 1 {
        let r = n - 1 - l;
        for (ln, rn) in [(false, false), (false, true), (true, false), (true, true)] {
            let rc = c[r][d][usize::from(rn)];
            let block = mul(c[l][d][usize::from(ln)], rc)?;
            if rank < block {
                let ar = rank / rc;
                let br = rank % rc;
                let a = if ln { unrank_new(l, d, ar, c)? } else { unrank_old(l, d, ar, c)? };
                let b = if rn { unrank_new(r, d, br, c)? } else { unrank_old(r, d, br, c)? };
                return Ok(F3m5LambdaExpr::EqAtom(Box::new(a), Box::new(b)));
            }
            rank -= block;
        }
    }

    for t in 1..n.saturating_sub(2) {
        for y in 1..n - 1 - t {
            let z = n - 1 - t - y;
            if z == 0 {
                continue;
            }
            for mask in 1u8..=7 {
                let tn = mask & 4 != 0;
                let yn = mask & 2 != 0;
                let zn = mask & 1 != 0;
                let yc = c[y][d][usize::from(yn)];
                let zc = c[z][d][usize::from(zn)];
                let yz = mul(yc, zc)?;
                let block = mul(c[t][d][usize::from(tn)], yz)?;
                if rank < block {
                    let tr = rank / yz;
                    let rem = rank % yz;
                    let decode = |nodes, is_new, rr| {
                        if is_new {
                            unrank_new(nodes, d, rr, c)
                        } else {
                            unrank_old(nodes, d, rr, c)
                        }
                    };
                    return Ok(F3m5LambdaExpr::IfAtom(
                        Box::new(decode(t, tn, tr)?),
                        Box::new(decode(y, yn, rem / zc)?),
                        Box::new(decode(z, zn, rem % zc)?),
                    ));
                }
                rank -= block;
            }
        }
    }
    Err(Frame3MicroEnumerateError::RankOutOfRange)
}

fn rank_new(
    e: &F3m5LambdaExpr,
    d: usize,
    c: &Counts,
) -> Result<u128, Frame3MicroEnumerateError> {
    if !e.contains_extension() {
        return Err(Frame3MicroEnumerateError::RankOutOfRange);
    }
    let n = e.node_count();
    if n == 1 {
        return match e {
            F3m5LambdaExpr::Bound(i) if usize::from(*i) < d => Ok(u128::from(*i)),
            _ => Err(Frame3MicroEnumerateError::RankOutOfRange),
        };
    }

    if let F3m5LambdaExpr::Atom(x) = e {
        if x.contains_extension() {
            return rank_new(x, d, c);
        }
    }
    let mut rank = c[n - 1][d][1];

    if let F3m5LambdaExpr::Lambda(body) = e {
        if body.contains_extension() {
            return add(rank, rank_new(body, d + 1, c)?);
        }
        rank = add(rank, c[n - 1][d + 1][1])?;
        return add(rank, rank_old(body, d + 1, c)?);
    }
    rank = add(rank, total(c, n - 1, d + 1)?)?;

    let binary = match e {
        F3m5LambdaExpr::Cons(a, b) => Some((0usize, a, b)),
        F3m5LambdaExpr::CarCons(a, b) => Some((1usize, a, b)),
        F3m5LambdaExpr::CdrCons(a, b) => Some((2usize, a, b)),
        _ => None,
    };
    for family in 0..3 {
        for l in 1..n - 1 {
            let r = n - 1 - l;
            for (ln, rn) in [(false, true), (true, false), (true, true)] {
                let block = mul(c[l][d][usize::from(ln)], c[r][d][usize::from(rn)])?;
                if let Some((actual_family, a, b)) = binary {
                    if family == actual_family
                        && l == a.node_count()
                        && (ln, rn) == (a.contains_extension(), b.contains_extension())
                    {
                        let ar = if ln { rank_new(a, d, c)? } else { rank_old(a, d, c)? };
                        let br = if rn { rank_new(b, d, c)? } else { rank_old(b, d, c)? };
                        rank = add(rank, mul(ar, c[r][d][usize::from(rn)])?)?;
                        return add(rank, br);
                    }
                }
                rank = add(rank, block)?;
            }
        }
    }

    for l in 1..n - 1 {
        let r = n - 1 - l;
        for (ln, rn) in [(false, false), (false, true), (true, false), (true, true)] {
            let block = mul(c[l][d][usize::from(ln)], c[r][d][usize::from(rn)])?;
            if let F3m5LambdaExpr::EqAtom(a, b) = e {
                if l == a.node_count()
                    && (ln, rn) == (a.contains_extension(), b.contains_extension())
                {
                    let ar = if ln { rank_new(a, d, c)? } else { rank_old(a, d, c)? };
                    let br = if rn { rank_new(b, d, c)? } else { rank_old(b, d, c)? };
                    rank = add(rank, mul(ar, c[r][d][usize::from(rn)])?)?;
                    return add(rank, br);
                }
            }
            rank = add(rank, block)?;
        }
    }

    if let F3m5LambdaExpr::IfAtom(a, b, z) = e {
        let actual_nodes = (a.node_count(), b.node_count(), z.node_count());
        let actual_mask = (u8::from(a.contains_extension()) << 2)
            | (u8::from(b.contains_extension()) << 1)
            | u8::from(z.contains_extension());
        for t in 1..n.saturating_sub(2) {
            for y in 1..n - 1 - t {
                let zz = n - 1 - t - y;
                if zz == 0 {
                    continue;
                }
                for mask in 1u8..=7 {
                    let tn = mask & 4 != 0;
                    let yn = mask & 2 != 0;
                    let zn = mask & 1 != 0;
                    let block = mul(
                        mul(c[t][d][usize::from(tn)], c[y][d][usize::from(yn)])?,
                        c[zz][d][usize::from(zn)],
                    )?;
                    if (t, y, zz) == actual_nodes && mask == actual_mask {
                        let rr = |x: &F3m5LambdaExpr, is_new| {
                            if is_new { rank_new(x, d, c) } else { rank_old(x, d, c) }
                        };
                        let yz = mul(c[y][d][usize::from(yn)], c[zz][d][usize::from(zn)])?;
                        rank = add(rank, mul(rr(a, tn)?, yz)?)?;
                        rank = add(rank, mul(rr(b, yn)?, c[zz][d][usize::from(zn)])?)?;
                        return add(rank, rr(z, zn)?);
                    }
                    rank = add(rank, block)?;
                }
            }
        }
    }
    Err(Frame3MicroEnumerateError::RankOutOfRange)
}

pub fn decode_candidate(payload: [u8; 5]) -> Result<F3m5LambdaExpr, Frame3MicroEnumerateError> {
    let mut rank = 0u128;
    for b in payload {
        rank = (rank << 8) | u128::from(b);
    }
    let c = counts();
    let mut remaining = rank;
    for n in 1..=MAX_NODES {
        let block = c[n][0][1];
        if remaining < block {
            return unrank_new(n, 0, remaining, c);
        }
        remaining -= block;
    }
    Err(Frame3MicroEnumerateError::RankOutOfRange)
}

pub fn encode_candidate(e: &F3m5LambdaExpr) -> Result<[u8; 5], Frame3MicroEnumerateError> {
    if !e.contains_extension() || e.node_count() > MAX_NODES {
        return Err(Frame3MicroEnumerateError::RankOutOfRange);
    }
    let c = counts();
    let mut rank = 0;
    for n in 1..e.node_count() {
        rank = add(rank, c[n][0][1])?;
    }
    rank = add(rank, rank_new(e, 0, c)?)?;
    if rank >= F3M5_LAMBDA_CAPACITY {
        return Err(Frame3MicroEnumerateError::RankOutOfRange);
    }

    let mut out = [0u8; 5];
    for b in out.iter_mut().rev() {
        *b = rank as u8;
        rank >>= 8;
    }
    Ok(out)
}

fn atom(v: &F3m5LambdaValue) -> bool {
    !matches!(v, F3m5LambdaValue::Pair(_, _))
}

fn eval_env(
    e: &F3m5LambdaExpr,
    env: &[F3m5LambdaValue],
) -> Result<F3m5LambdaValue, Frame3MicroEnumerateError> {
    match e {
        F3m5LambdaExpr::Zero | F3m5LambdaExpr::QuoteZero => Ok(F3m5LambdaValue::Bit(false)),
        F3m5LambdaExpr::One | F3m5LambdaExpr::QuoteOne => Ok(F3m5LambdaValue::Bit(true)),
        F3m5LambdaExpr::Nil | F3m5LambdaExpr::QuoteNil => Ok(F3m5LambdaValue::Nil),
        F3m5LambdaExpr::Bound(i) => env
            .get(usize::from(*i))
            .cloned()
            .ok_or(Frame3MicroEnumerateError::RankOutOfRange),
        F3m5LambdaExpr::Atom(x) => Ok(F3m5LambdaValue::Bit(atom(&eval_env(x, env)?))),
        F3m5LambdaExpr::Cons(a, b) => Ok(F3m5LambdaValue::Pair(
            Box::new(eval_env(a, env)?),
            Box::new(eval_env(b, env)?),
        )),
        F3m5LambdaExpr::CarCons(a, b) => {
            let a = eval_env(a, env)?;
            let _ = eval_env(b, env)?;
            Ok(a)
        }
        F3m5LambdaExpr::CdrCons(a, b) => {
            let _ = eval_env(a, env)?;
            eval_env(b, env)
        }
        F3m5LambdaExpr::EqAtom(a, b) => {
            Ok(F3m5LambdaValue::Bit(atom(&eval_env(a, env)?) == atom(&eval_env(b, env)?)))
        }
        F3m5LambdaExpr::Lambda(body) => Ok(F3m5LambdaValue::Closure {
            body: body.clone(),
            env: env.to_vec(),
        }),
        F3m5LambdaExpr::IfAtom(test, yes, no) => {
            if atom(&eval_env(test, env)?) { eval_env(yes, env) } else { eval_env(no, env) }
        }
    }
}

pub fn eval_candidate(e: &F3m5LambdaExpr) -> Result<F3m5LambdaValue, Frame3MicroEnumerateError> {
    eval_env(e, &[])
}

pub fn apply_candidate(
    closure: F3m5LambdaValue,
    arg: F3m5LambdaValue,
) -> Result<F3m5LambdaValue, Frame3MicroEnumerateError> {
    let F3m5LambdaValue::Closure { body, env } = closure else {
        return Err(Frame3MicroEnumerateError::RankOutOfRange);
    };
    let mut bound = Vec::with_capacity(env.len() + 1);
    bound.push(arg);
    bound.extend(env);
    eval_env(&body, &bound)
}


/// Internal evaluator-only binder token used solely to allocate one call-frame
/// slot for a nameless lambda.  F3M-5λ semantic identity never contains this
/// string: every bound occurrence is already an ExprKind::Local coordinate.
///
/// A NUL prefix keeps this token outside ordinary parsed source spellings.  If
/// this debug/arity token changes, the physical five-byte mapping and lowered
/// Local coordinates remain identical.
const RUNTIME_BINDER_SENTINEL: &str = "\0f3m-debruijn-slot";

fn runtime_expr(kind: ExprKind) -> Expr {
    Expr {
        kind,
        span: Span::default(),
    }
}

fn runtime_list(items: Vec<Expr>) -> Expr {
    runtime_expr(ExprKind::List(Rc::from(items.into_boxed_slice())))
}

fn runtime_d1(bit: bool) -> Expr {
    runtime_expr(ExprKind::DomainIdentity(DomainIdentity::D1(
        PredicateBit::from_word(Bit1::new(u8::from(bit)).expect("D1 bit")),
    )))
}

fn runtime_d3(bits: u8, arguments: Vec<Expr>) -> Expr {
    runtime_expr(ExprKind::DomainCall(
        CoreDomainIdentity::D3(Bija3::from_word(Bit3::new(bits).expect("D3 word"))),
        Rc::from(arguments.into_boxed_slice()),
    ))
}

fn runtime_lambda(body: Expr) -> Expr {
    let parameter = runtime_expr(ExprKind::Symbol(Rc::from(RUNTIME_BINDER_SENTINEL)));
    let parameters = runtime_list(vec![parameter]);
    runtime_expr(ExprKind::DomainCall(
        CoreDomainIdentity::D4(CoreD4::from_word(Bit4::new(0b0010).expect("D4 LAMBDA"))),
        Rc::from(vec![parameters, body].into_boxed_slice()),
    ))
}

fn lower_runtime(
    expression: &F3m5LambdaExpr,
    lexical_depth: usize,
) -> Result<Expr, LanguageError> {
    Ok(match expression {
        F3m5LambdaExpr::Zero => runtime_d1(false),
        F3m5LambdaExpr::One => runtime_d1(true),
        F3m5LambdaExpr::Nil => runtime_list(vec![]),
        F3m5LambdaExpr::QuoteZero => runtime_d3(0b001, vec![runtime_d1(false)]),
        F3m5LambdaExpr::QuoteOne => runtime_d3(0b001, vec![runtime_d1(true)]),
        F3m5LambdaExpr::QuoteNil => runtime_d3(0b001, vec![runtime_list(vec![])]),
        F3m5LambdaExpr::Bound(index) => {
            if usize::from(*index) >= lexical_depth {
                return Err(LanguageError::new(
                    ErrorKind::InvalidForm,
                    format!(
                        "F3M-5λ BOUND({index}) is free at lexical depth {lexical_depth}"
                    ),
                    Span::default(),
                ));
            }
            runtime_expr(ExprKind::Local {
                depth: u32::from(*index),
                index: 0,
            })
        }
        F3m5LambdaExpr::Atom(value) => {
            runtime_d3(0b010, vec![lower_runtime(value, lexical_depth)?])
        }
        F3m5LambdaExpr::Cons(left, right) => runtime_d3(
            0b111,
            vec![
                lower_runtime(left, lexical_depth)?,
                lower_runtime(right, lexical_depth)?,
            ],
        ),
        F3m5LambdaExpr::CarCons(left, right) => {
            let cons = runtime_d3(
                0b111,
                vec![
                    lower_runtime(left, lexical_depth)?,
                    lower_runtime(right, lexical_depth)?,
                ],
            );
            runtime_d3(0b100, vec![cons])
        }
        F3m5LambdaExpr::CdrCons(left, right) => {
            let cons = runtime_d3(
                0b111,
                vec![
                    lower_runtime(left, lexical_depth)?,
                    lower_runtime(right, lexical_depth)?,
                ],
            );
            runtime_d3(0b011, vec![cons])
        }
        F3m5LambdaExpr::EqAtom(left, right) => {
            let left = runtime_d3(0b010, vec![lower_runtime(left, lexical_depth)?]);
            let right = runtime_d3(0b010, vec![lower_runtime(right, lexical_depth)?]);
            runtime_d3(0b101, vec![left, right])
        }
        F3m5LambdaExpr::Lambda(body) => {
            runtime_lambda(lower_runtime(body, lexical_depth + 1)?)
        }
        F3m5LambdaExpr::IfAtom(test, yes, no) => {
            let predicate = runtime_d3(0b010, vec![lower_runtime(test, lexical_depth)?]);
            let yes_clause = runtime_list(vec![predicate, lower_runtime(yes, lexical_depth)?]);
            let no_clause = runtime_list(vec![runtime_d1(true), lower_runtime(no, lexical_depth)?]);
            runtime_d3(0b110, vec![yes_clause, no_clause])
        }
    })
}

/// Lower the nameless five-byte candidate directly into SENS's existing
/// numeric lexical-slot AST.  Bound-variable names never enter the result:
/// BOUND(n) becomes Local { depth: n, index: 0 }.
///
/// The only temporary string is the evaluator-private lambda-list sentinel
/// required by the current Closure arity/frame allocator.  It is never
/// referenced by the body and therefore cannot affect semantic identity.
pub fn lower_candidate_to_sens_expr(
    expression: &F3m5LambdaExpr,
) -> Result<Expr, LanguageError> {
    lower_runtime(expression, 0)
}

/// Evaluate a decoded F3M-5λ term with the real SENS evaluator, not the small
/// research evaluator above.
pub fn eval_candidate_with_sens_runtime(
    expression: &F3m5LambdaExpr,
    session: &mut Session,
) -> Result<Value, LanguageError> {
    let lowered = lower_candidate_to_sens_expr(expression)?;
    crate::eval_expr(&lowered, &session.environment)
}

/// Decode an exact five-byte F3M-5λ payload and run it through the real SENS
/// evaluator.  Decode errors remain distinct from language-evaluation errors.
pub fn decode_and_eval_with_sens_runtime(
    payload: [u8; 5],
    session: &mut Session,
) -> Result<Value, String> {
    let expression = decode_candidate(payload).map_err(|error| format!("{error}"))?;
    eval_candidate_with_sens_runtime(&expression, session).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame3_micro_enumerator::CAP_ALL_F3M5;

    #[test]
    fn closed_space_fills_all_40_bits_by_node_13() {
        let c = counts();
        let through_12 = (1..=12).map(|n| c[n][0][1]).sum::<u128>();
        let through_13 = through_12 + c[13][0][1];
        assert_eq!(through_12, 298_460_447_325);
        assert_eq!(through_13, 3_706_360_960_907);
        assert!(through_12 < F3M5_LAMBDA_CAPACITY);
        assert!(through_13 >= F3M5_LAMBDA_CAPACITY);
    }

    #[test]
    fn zero_payload_is_lambda_identity() {
        let id = F3m5LambdaExpr::Lambda(Box::new(F3m5LambdaExpr::Bound(0)));
        assert_eq!(decode_candidate([0; 5]).unwrap(), id);
        assert_eq!(encode_candidate(&id).unwrap(), [0; 5]);
        assert_eq!(id.human_projection(), "λ.BOUND(0)");
    }

    #[test]
    fn identity_beta_returns_argument_exactly() {
        let id = F3m5LambdaExpr::Lambda(Box::new(F3m5LambdaExpr::Bound(0)));
        let closure = eval_candidate(&id).unwrap();
        for arg in [
            F3m5LambdaValue::Bit(false),
            F3m5LambdaValue::Bit(true),
            F3m5LambdaValue::Nil,
            F3m5LambdaValue::Pair(
                Box::new(F3m5LambdaValue::Bit(true)),
                Box::new(F3m5LambdaValue::Nil),
            ),
        ] {
            assert_eq!(apply_candidate(closure.clone(), arg.clone()).unwrap(), arg);
        }
    }

    #[test]
    fn nested_indices_distinguish_nearest_from_outer() {
        let nearest = F3m5LambdaExpr::Lambda(Box::new(F3m5LambdaExpr::Lambda(Box::new(
            F3m5LambdaExpr::Bound(0),
        ))));
        let outer = F3m5LambdaExpr::Lambda(Box::new(F3m5LambdaExpr::Lambda(Box::new(
            F3m5LambdaExpr::Bound(1),
        ))));
        let one = F3m5LambdaValue::Bit(true);
        let zero = F3m5LambdaValue::Bit(false);

        let n = apply_candidate(eval_candidate(&nearest).unwrap(), one.clone()).unwrap();
        assert_eq!(apply_candidate(n, zero.clone()).unwrap(), zero);

        let o = apply_candidate(eval_candidate(&outer).unwrap(), one.clone()).unwrap();
        assert_eq!(apply_candidate(o, zero).unwrap(), one);
        assert_ne!(encode_candidate(&nearest).unwrap(), encode_candidate(&outer).unwrap());
    }

    #[test]
    fn free_bound_reference_fails_closed() {
        assert!(encode_candidate(&F3m5LambdaExpr::Bound(0)).is_err());
        assert!(eval_candidate(&F3m5LambdaExpr::Bound(0)).is_err());
    }

    #[test]
    fn stratified_full_window_roundtrip() {
        for value in [0u64, 1, 6, 0x12_34_56_78, 0x80_00_00_00_00, 0xde_ad_be_ef_01, 0xff_ff_ff_ff_ff] {
            let p = [
                ((value >> 32) & 0xff) as u8,
                ((value >> 24) & 0xff) as u8,
                ((value >> 16) & 0xff) as u8,
                ((value >> 8) & 0xff) as u8,
                (value & 0xff) as u8,
            ];
            assert_eq!(encode_candidate(&decode_candidate(p).unwrap()).unwrap(), p);
        }
        for high in 0u64..=u16::MAX as u64 {
            let low = ((high * 0x9e37) ^ 0x6d_b3_21) & 0xff_ff_ff;
            let v = (high << 24) | low;
            let p = [
                ((v >> 32) & 0xff) as u8,
                ((v >> 24) & 0xff) as u8,
                ((v >> 16) & 0xff) as u8,
                ((v >> 8) & 0xff) as u8,
                (v & 0xff) as u8,
            ];
            assert_eq!(encode_candidate(&decode_candidate(p).unwrap()).unwrap(), p);
        }
    }

    #[test]
    fn canonical_i_k_ki_have_unique_five_byte_payloads_and_beta_behavior() {
        let i = F3m5LambdaExpr::Lambda(Box::new(F3m5LambdaExpr::Bound(0)));
        let k = F3m5LambdaExpr::Lambda(Box::new(F3m5LambdaExpr::Lambda(Box::new(
            F3m5LambdaExpr::Bound(1),
        ))));
        let ki = F3m5LambdaExpr::Lambda(Box::new(F3m5LambdaExpr::Lambda(Box::new(
            F3m5LambdaExpr::Bound(0),
        ))));

        let pi = encode_candidate(&i).expect("I must fit F3M-5λ");
        let pk = encode_candidate(&k).expect("K must fit F3M-5λ");
        let pki = encode_candidate(&ki).expect("KI must fit F3M-5λ");
        assert_ne!(pi, pk);
        assert_ne!(pi, pki);
        assert_ne!(pk, pki);
        assert_eq!(decode_candidate(pi).unwrap(), i);
        assert_eq!(decode_candidate(pk).unwrap(), k);
        assert_eq!(decode_candidate(pki).unwrap(), ki);

        let one = F3m5LambdaValue::Bit(true);
        let zero = F3m5LambdaValue::Bit(false);

        let i_value = apply_candidate(eval_candidate(&i).unwrap(), one.clone()).unwrap();
        assert_eq!(i_value, one);

        let k1 = apply_candidate(eval_candidate(&k).unwrap(), one.clone()).unwrap();
        assert_eq!(apply_candidate(k1, zero.clone()).unwrap(), one);

        let ki1 = apply_candidate(eval_candidate(&ki).unwrap(), one).unwrap();
        assert_eq!(apply_candidate(ki1, zero.clone()).unwrap(), zero);

        let hex = |payload: [u8; 5]| {
            payload
                .iter()
                .map(|byte| format!("{byte:02X}"))
                .collect::<String>()
        };
        eprintln!(
            "F3M5L_I={}\tK={}\tKI={}",
            hex(pi),
            hex(pk),
            hex(pki)
        );
    }

    #[test]
    fn five_byte_lambda_behavioral_trophy_catalog() {
        fn hex(payload: [u8; 5]) -> String {
            payload.iter().map(|b| format!("{b:02X}")).collect::<String>()
        }
        fn trophy(name: &str, expr: F3m5LambdaExpr) -> [u8; 5] {
            let payload = encode_candidate(&expr).expect("trophy must fit F3M-5λ");
            assert_eq!(decode_candidate(payload).unwrap(), expr);
            eprintln!(
                "F3M5L_TROPHY\t{name}\t{}\tnodes={}\tcap={:02X}\t{}",
                hex(payload),
                expr.node_count(),
                expr.capability_mask(),
                expr.human_projection()
            );
            payload
        }

        let i = F3m5LambdaExpr::Lambda(Box::new(F3m5LambdaExpr::Bound(0)));
        let k = F3m5LambdaExpr::Lambda(Box::new(F3m5LambdaExpr::Lambda(Box::new(
            F3m5LambdaExpr::Bound(1),
        ))));
        let ki = F3m5LambdaExpr::Lambda(Box::new(F3m5LambdaExpr::Lambda(Box::new(
            F3m5LambdaExpr::Bound(0),
        ))));
        let pair = F3m5LambdaExpr::Lambda(Box::new(F3m5LambdaExpr::Lambda(Box::new(
            F3m5LambdaExpr::Cons(
                Box::new(F3m5LambdaExpr::Bound(1)),
                Box::new(F3m5LambdaExpr::Bound(0)),
            ),
        ))));
        let swap_pair = F3m5LambdaExpr::Lambda(Box::new(F3m5LambdaExpr::Lambda(Box::new(
            F3m5LambdaExpr::Cons(
                Box::new(F3m5LambdaExpr::Bound(0)),
                Box::new(F3m5LambdaExpr::Bound(1)),
            ),
        ))));
        let atom_pred = F3m5LambdaExpr::Lambda(Box::new(F3m5LambdaExpr::Atom(Box::new(
            F3m5LambdaExpr::Bound(0),
        ))));
        let first = F3m5LambdaExpr::Lambda(Box::new(F3m5LambdaExpr::Lambda(Box::new(
            F3m5LambdaExpr::CarCons(
                Box::new(F3m5LambdaExpr::Bound(1)),
                Box::new(F3m5LambdaExpr::Bound(0)),
            ),
        ))));
        let second = F3m5LambdaExpr::Lambda(Box::new(F3m5LambdaExpr::Lambda(Box::new(
            F3m5LambdaExpr::CdrCons(
                Box::new(F3m5LambdaExpr::Bound(1)),
                Box::new(F3m5LambdaExpr::Bound(0)),
            ),
        ))));
        let same_atom_kind = F3m5LambdaExpr::Lambda(Box::new(F3m5LambdaExpr::Lambda(Box::new(
            F3m5LambdaExpr::EqAtom(
                Box::new(F3m5LambdaExpr::Bound(1)),
                Box::new(F3m5LambdaExpr::Bound(0)),
            ),
        ))));
        let atom_to_bit = F3m5LambdaExpr::Lambda(Box::new(F3m5LambdaExpr::IfAtom(
            Box::new(F3m5LambdaExpr::Bound(0)),
            Box::new(F3m5LambdaExpr::QuoteOne),
            Box::new(F3m5LambdaExpr::QuoteZero),
        )));

        let _ = trophy("I", i.clone());
        let _ = trophy("K", k.clone());
        let _ = trophy("KI", ki.clone());
        let _ = trophy("PAIR", pair.clone());
        let _ = trophy("SWAP_PAIR", swap_pair.clone());
        let _ = trophy("ATOM_PRED", atom_pred.clone());
        let _ = trophy("FIRST_VIA_CAR_CONS", first.clone());
        let _ = trophy("SECOND_VIA_CDR_CONS", second.clone());
        let _ = trophy("SAME_ATOM_KIND", same_atom_kind.clone());
        let _ = trophy("ATOM_TO_BIT_COND", atom_to_bit.clone());

        let one = F3m5LambdaValue::Bit(true);
        let zero = F3m5LambdaValue::Bit(false);
        let nil = F3m5LambdaValue::Nil;
        let pair_value = F3m5LambdaValue::Pair(Box::new(one.clone()), Box::new(nil.clone()));

        assert_eq!(
            apply_candidate(eval_candidate(&i).unwrap(), pair_value.clone()).unwrap(),
            pair_value
        );

        let k1 = apply_candidate(eval_candidate(&k).unwrap(), one.clone()).unwrap();
        assert_eq!(apply_candidate(k1, zero.clone()).unwrap(), one);

        let ki1 = apply_candidate(eval_candidate(&ki).unwrap(), one.clone()).unwrap();
        assert_eq!(apply_candidate(ki1, zero.clone()).unwrap(), zero);

        let pair1 = apply_candidate(eval_candidate(&pair).unwrap(), one.clone()).unwrap();
        assert_eq!(
            apply_candidate(pair1, zero.clone()).unwrap(),
            F3m5LambdaValue::Pair(Box::new(one.clone()), Box::new(zero.clone()))
        );

        let swap1 = apply_candidate(eval_candidate(&swap_pair).unwrap(), one.clone()).unwrap();
        assert_eq!(
            apply_candidate(swap1, zero.clone()).unwrap(),
            F3m5LambdaValue::Pair(Box::new(zero.clone()), Box::new(one.clone()))
        );

        assert_eq!(
            apply_candidate(eval_candidate(&atom_pred).unwrap(), pair_value.clone()).unwrap(),
            F3m5LambdaValue::Bit(false)
        );
        assert_eq!(
            apply_candidate(eval_candidate(&atom_pred).unwrap(), one.clone()).unwrap(),
            F3m5LambdaValue::Bit(true)
        );

        let first1 = apply_candidate(eval_candidate(&first).unwrap(), one.clone()).unwrap();
        assert_eq!(apply_candidate(first1, zero.clone()).unwrap(), one);

        let second1 = apply_candidate(eval_candidate(&second).unwrap(), one.clone()).unwrap();
        assert_eq!(apply_candidate(second1, zero.clone()).unwrap(), zero);

        let eq1 = apply_candidate(eval_candidate(&same_atom_kind).unwrap(), one.clone()).unwrap();
        assert_eq!(
            apply_candidate(eq1, zero.clone()).unwrap(),
            F3m5LambdaValue::Bit(true)
        );
        let eq2 = apply_candidate(eval_candidate(&same_atom_kind).unwrap(), pair_value.clone()).unwrap();
        assert_eq!(
            apply_candidate(eq2, one.clone()).unwrap(),
            F3m5LambdaValue::Bit(false)
        );

        assert_eq!(
            apply_candidate(eval_candidate(&atom_to_bit).unwrap(), one).unwrap(),
            F3m5LambdaValue::Bit(true)
        );
        assert_eq!(
            apply_candidate(eval_candidate(&atom_to_bit).unwrap(), pair_value).unwrap(),
            F3m5LambdaValue::Bit(false)
        );
    }

    #[test]
    fn zero_payload_runs_as_real_sens_lambda_identity_without_a_source_name() {
        let candidate = decode_candidate([0; 5]).expect("five-byte identity");
        let lowered = lower_candidate_to_sens_expr(&candidate).expect("closed nameless lambda");

        let ExprKind::DomainCall(lambda, arguments) = &lowered.kind else {
            panic!("F3M-5λ identity must lower to real D4 LAMBDA");
        };
        assert_eq!((lambda.width(), lambda.packed_bits()), (4, 0b0010));
        let ExprKind::List(parameters) = &arguments[0].kind else {
            panic!("runtime lambda-list");
        };
        assert_eq!(parameters.len(), 1);
        let ExprKind::Local { depth, index } = arguments[1].kind else {
            panic!("λ.BOUND(0) body must already be a numeric Local");
        };
        assert_eq!((depth, index), (0, 0));

        for (argument, expected) in [
            (runtime_d1(false), "0"),
            (runtime_d1(true), "1"),
            (runtime_list(vec![]), "()"),
            (runtime_d3(0b111, vec![runtime_d1(true), runtime_list(vec![])]), "(1)"),
        ] {
            let call = runtime_list(vec![lowered.clone(), argument]);
            let session = Session::bare();
            let actual = crate::eval_expr(&call, &session.environment)
                .expect("real SENS evaluator must execute nameless identity");
            assert_eq!(actual.to_string(), expected);
        }
    }

    #[test]
    fn k_and_ki_beta_reduce_through_real_sens_nested_closures() {
        let k = decode_candidate([0x00, 0x00, 0x00, 0x00, 0x10]).expect("K");
        let ki = decode_candidate([0x00, 0x00, 0x00, 0x00, 0x0f]).expect("KI");

        for (term, expected) in [(k, "1"), (ki, "0")] {
            let lowered = lower_candidate_to_sens_expr(&term).expect("closed K-family term");
            let first = runtime_list(vec![lowered, runtime_d1(true)]);
            let second = runtime_list(vec![first, runtime_d1(false)]);
            let session = Session::bare();
            let actual = crate::eval_expr(&second, &session.environment)
                .expect("real SENS nested closure execution");
            assert_eq!(actual.to_string(), expected);
        }
    }

    #[test]
    fn tracked_all_basis_payload_executes_on_real_sens_evaluator() {
        const ALL: &[u8; 5] =
            include_bytes!("../../../research/framed3/fixtures/f3m5-all-d1-d3-lambda.f3m");
        let candidate = decode_candidate(*ALL).expect("all-basis five-byte witness");
        assert_eq!(candidate.capability_mask(), CAP_ALL_F3M5);

        let mut session = Session::bare();
        let actual = eval_candidate_with_sens_runtime(&candidate, &mut session)
            .expect("all D1-D3 capabilities plus real lambda run in canonical evaluator");
        assert_eq!(actual.as_predicate_bit(), Some(false));
    }

    #[test]
    fn free_debruijn_reference_fails_before_real_sens_evaluation() {
        let error = lower_candidate_to_sens_expr(&F3m5LambdaExpr::Bound(0))
            .expect_err("root BOUND(0) is not closed");
        assert_eq!(error.kind, ErrorKind::InvalidForm);
    }

    #[test]
    fn tracked_five_byte_lambda_trophies_are_exact_physical_payloads() {
        const I: &[u8; 5] = include_bytes!("../../../research/framed3/fixtures/f3m5-lambda-i.f3m");
        const K: &[u8; 5] = include_bytes!("../../../research/framed3/fixtures/f3m5-lambda-k.f3m");
        const KI: &[u8; 5] = include_bytes!("../../../research/framed3/fixtures/f3m5-lambda-ki.f3m");
        const ALL: &[u8; 5] = include_bytes!("../../../research/framed3/fixtures/f3m5-all-d1-d3-lambda.f3m");

        assert_eq!(*I, [0x00, 0x00, 0x00, 0x00, 0x00]);
        assert_eq!(*K, [0x00, 0x00, 0x00, 0x00, 0x10]);
        assert_eq!(*KI, [0x00, 0x00, 0x00, 0x00, 0x0f]);
        assert_eq!(*ALL, [0x05, 0x61, 0x69, 0xcb, 0x4a]);

        let i = decode_candidate(*I).expect("tracked I");
        let k = decode_candidate(*K).expect("tracked K");
        let ki = decode_candidate(*KI).expect("tracked KI");
        let all = decode_candidate(*ALL).expect("tracked all-capability witness");

        assert_eq!(i, F3m5LambdaExpr::Lambda(Box::new(F3m5LambdaExpr::Bound(0))));
        assert_eq!(encode_candidate(&i).unwrap(), *I);
        assert_eq!(encode_candidate(&k).unwrap(), *K);
        assert_eq!(encode_candidate(&ki).unwrap(), *KI);
        assert_eq!(encode_candidate(&all).unwrap(), *ALL);
        assert_eq!(all.capability_mask(), CAP_ALL_F3M5);

        let one = F3m5LambdaValue::Bit(true);
        let zero = F3m5LambdaValue::Bit(false);
        assert_eq!(apply_candidate(eval_candidate(&i).unwrap(), one.clone()).unwrap(), one);
        let k1 = apply_candidate(eval_candidate(&k).unwrap(), one.clone()).unwrap();
        assert_eq!(apply_candidate(k1, zero.clone()).unwrap(), one);
        let ki1 = apply_candidate(eval_candidate(&ki).unwrap(), one).unwrap();
        assert_eq!(apply_candidate(ki1, zero.clone()).unwrap(), zero);
    }

    #[test]
    fn alpha_equivalent_surface_names_collapse_to_one_nameless_identity() {
        // The candidate has no symbol/name constructor for binders at all:
        // λx.x, λy.y and λfoo.foo all lower to this single closed term.
        let canonical_identity =
            F3m5LambdaExpr::Lambda(Box::new(F3m5LambdaExpr::Bound(0)));
        assert_eq!(canonical_identity.human_projection(), "λ.BOUND(0)");
        assert_eq!(encode_candidate(&canonical_identity).unwrap(), [0; 5]);
    }

    #[test]
    fn one_program_carries_all_d1_d3_capabilities_plus_real_lambda() {
        let e = F3m5LambdaExpr::IfAtom(
            Box::new(F3m5LambdaExpr::EqAtom(
                Box::new(F3m5LambdaExpr::Lambda(Box::new(F3m5LambdaExpr::Bound(0)))),
                Box::new(F3m5LambdaExpr::QuoteOne),
            )),
            Box::new(F3m5LambdaExpr::CarCons(
                Box::new(F3m5LambdaExpr::QuoteZero),
                Box::new(F3m5LambdaExpr::CdrCons(
                    Box::new(F3m5LambdaExpr::QuoteOne),
                    Box::new(F3m5LambdaExpr::QuoteNil),
                )),
            )),
            Box::new(F3m5LambdaExpr::QuoteNil),
        );
        assert_eq!(e.capability_mask(), CAP_ALL_F3M5);
        let p = encode_candidate(&e).unwrap();
        assert_eq!(decode_candidate(p).unwrap(), e);
        eprintln!(
            "F3M5L_ALL_D1_D3_LAMBDA={}\tnodes={}\t{}",
            p.iter().map(|b| format!("{b:02X}")).collect::<String>(),
            e.node_count(),
            e.human_projection()
        );
    }
}
