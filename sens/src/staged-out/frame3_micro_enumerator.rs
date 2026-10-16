//! Dense Frame3-Micro enumerator for 1/2/3/4-byte payloads plus prefix-stable F3M-5.
//!
//! Canonical order is over a deliberately total, closed D3 microgrammar:
//! literals 0/1/NIL, QUOTE of those literals, ATOM, CONS,
//! CAR(CONS(...)), CDR(CONS(...)), and a total IF-ATOM expansion through
//! exact D3 COND. Every constructor is chosen so decoded programs are closed
//! and do not require host names or dynamic bindings.
//!
//! The global enumeration is ordered first by expression node count, then by
//! constructor/split/rank order. Physical profiles are disjoint windows:
//!   F3M-1 = global ranks [0, 256)
//!   F3M-2 = next 65_536 ranks
//!   F3M-3 = next 16_777_216 ranks
//!   F3M-4 = next 4_294_967_296 legacy ranks
//!   F3M-5 = independent 2^40 extension ranks containing EQ and/or LAMBDA
//! Thus every payload value in each admitted width names exactly one program,
//! with no reserved/magic code points and no aliases across widths.

use crate::frame3_micro::{frame3_micro_profile, Frame3MicroProfile};

const LEAF_COUNT: u128 = 6;
const F3M1_CAPACITY: u128 = 1u128 << 8;
const F3M2_CAPACITY: u128 = 1u128 << 16;
const F3M3_CAPACITY: u128 = 1u128 << 24;
const F3M4_CAPACITY: u128 = 1u128 << 32;
const F3M5_CAPACITY: u128 = crate::frame3_micro_v5::F3M5_CAPACITY;
const F3M2_OFFSET: u128 = F3M1_CAPACITY;
const F3M3_OFFSET: u128 = F3M1_CAPACITY + F3M2_CAPACITY;
const F3M4_OFFSET: u128 = F3M3_OFFSET + F3M3_CAPACITY;
const F3M_END: u128 = F3M4_OFFSET + F3M4_CAPACITY;
const COUNT_LIMIT: usize = 16;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Frame3MicroExpr {
    Zero,
    One,
    Nil,
    QuoteZero,
    QuoteOne,
    QuoteNil,
    Atom(Box<Frame3MicroExpr>),
    Cons(Box<Frame3MicroExpr>, Box<Frame3MicroExpr>),
    CarCons(Box<Frame3MicroExpr>, Box<Frame3MicroExpr>),
    CdrCons(Box<Frame3MicroExpr>, Box<Frame3MicroExpr>),
    /// Total D3 EQ witness: EQ is applied only to exact-D1 ATOM results.
    EqAtom(Box<Frame3MicroExpr>, Box<Frame3MicroExpr>),
    /// D4 LAMBDA with an empty parameter list and a closed body.
    Lambda0(Box<Frame3MicroExpr>),
    /// Total conditional:
    /// (COND ((ATOM test) then) (1 else))
    IfAtom(
        Box<Frame3MicroExpr>,
        Box<Frame3MicroExpr>,
        Box<Frame3MicroExpr>,
    ),
}

pub const CAP_QUOTE: u8 = 1 << 0;
pub const CAP_ATOM: u8 = 1 << 1;
pub const CAP_CONS: u8 = 1 << 2;
pub const CAP_CAR: u8 = 1 << 3;
pub const CAP_CDR: u8 = 1 << 4;
pub const CAP_COND: u8 = 1 << 5;
pub const CAP_EQ: u8 = 1 << 6;
pub const CAP_LAMBDA: u8 = 1 << 7;
/// Frozen six-capability mask used by F3M-1..4 proofs.
pub const CAP_ALL: u8 = CAP_QUOTE | CAP_ATOM | CAP_CONS | CAP_CAR | CAP_CDR | CAP_COND;
/// F3M-5 adds the missing D3 EQ capability plus D4 LAMBDA.
pub const CAP_ALL_F3M5: u8 = CAP_ALL | CAP_EQ | CAP_LAMBDA;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Frame3MicroEnumerateError {
    UnsupportedLength(usize),
    RankOutOfRange,
    ProgramOutsideF3M4,
    InternalCountOverflow,
}

impl std::fmt::Display for Frame3MicroEnumerateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for Frame3MicroEnumerateError {}

impl Frame3MicroExpr {
    pub fn node_count(&self) -> usize {
        match self {
            Self::Zero
            | Self::One
            | Self::Nil
            | Self::QuoteZero
            | Self::QuoteOne
            | Self::QuoteNil => 1,
            Self::Atom(value) | Self::Lambda0(value) => 1 + value.node_count(),
            Self::Cons(left, right)
            | Self::CarCons(left, right)
            | Self::CdrCons(left, right)
            | Self::EqAtom(left, right) => 1 + left.node_count() + right.node_count(),
            Self::IfAtom(test, then_value, else_value) => {
                1 + test.node_count() + then_value.node_count() + else_value.node_count()
            }
        }
    }

    pub fn exact_words(&self) -> Vec<&'static str> {
        match self {
            Self::Zero => vec!["0"],
            Self::One => vec!["1"],
            Self::Nil => vec!["000"],
            Self::QuoteZero => unary_call("001", vec!["0"]),
            Self::QuoteOne => unary_call("001", vec!["1"]),
            Self::QuoteNil => unary_call("001", vec!["000"]),
            Self::Atom(value) => unary_call("010", value.exact_words()),
            Self::Cons(left, right) => binary_call("111", left.exact_words(), right.exact_words()),
            Self::CarCons(left, right) => {
                unary_call("100", binary_call("111", left.exact_words(), right.exact_words()))
            }
            Self::CdrCons(left, right) => {
                unary_call("011", binary_call("111", left.exact_words(), right.exact_words()))
            }
            Self::EqAtom(left, right) => binary_call(
                "101",
                unary_call("010", left.exact_words()),
                unary_call("010", right.exact_words()),
            ),
            Self::Lambda0(body) => lambda0_call(body.exact_words()),
            Self::IfAtom(test, then_value, else_value) => {
                let predicate = unary_call("010", test.exact_words());
                let first_clause = list2(predicate, then_value.exact_words());
                let second_clause = list2(vec!["1"], else_value.exact_words());
                let mut out = vec!["10", "110"];
                out.extend(first_clause);
                out.extend(second_clause);
                out.push("01");
                out
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
            Self::Atom(value) => format!("ATOM({})", value.human_projection()),
            Self::Cons(left, right) => {
                format!("CONS({},{})", left.human_projection(), right.human_projection())
            }
            Self::CarCons(left, right) => format!(
                "CAR(CONS({},{}))",
                left.human_projection(),
                right.human_projection()
            ),
            Self::CdrCons(left, right) => format!(
                "CDR(CONS({},{}))",
                left.human_projection(),
                right.human_projection()
            ),
            Self::EqAtom(left, right) => format!(
                "EQ(ATOM({}),ATOM({}))",
                left.human_projection(),
                right.human_projection()
            ),
            Self::Lambda0(body) => format!("LAMBDA((),{})", body.human_projection()),
            Self::IfAtom(test, then_value, else_value) => format!(
                "IF-ATOM({},{},{})",
                test.human_projection(),
                then_value.human_projection(),
                else_value.human_projection()
            ),
        }
    }


    /// Semantic constructor classes exercised by this closed micro-program.
    ///
    /// CAR(CONS(...)) and CDR(CONS(...)) count both the selector and CONS,
    /// while IF-ATOM counts COND; its exact expansion also contains ATOM.
    pub fn capability_mask(&self) -> u8 {
        match self {
            Self::Zero | Self::One | Self::Nil => 0,
            Self::QuoteZero | Self::QuoteOne | Self::QuoteNil => CAP_QUOTE,
            Self::Atom(value) => CAP_ATOM | value.capability_mask(),
            Self::Cons(left, right) => {
                CAP_CONS | left.capability_mask() | right.capability_mask()
            }
            Self::CarCons(left, right) => {
                CAP_CAR | CAP_CONS | left.capability_mask() | right.capability_mask()
            }
            Self::CdrCons(left, right) => {
                CAP_CDR | CAP_CONS | left.capability_mask() | right.capability_mask()
            }
            Self::EqAtom(left, right) => {
                CAP_EQ | CAP_ATOM | left.capability_mask() | right.capability_mask()
            }
            Self::Lambda0(body) => CAP_LAMBDA | body.capability_mask(),
            Self::IfAtom(test, then_value, else_value) => {
                CAP_COND
                    | CAP_ATOM
                    | test.capability_mask()
                    | then_value.capability_mask()
                    | else_value.capability_mask()
            }
        }
    }
}

fn unary_call(head: &'static str, argument: Vec<&'static str>) -> Vec<&'static str> {
    let mut out = vec!["10", head];
    out.extend(argument);
    out.push("01");
    out
}

fn binary_call(
    head: &'static str,
    left: Vec<&'static str>,
    right: Vec<&'static str>,
) -> Vec<&'static str> {
    let mut out = vec!["10", head];
    out.extend(left);
    out.extend(right);
    out.push("01");
    out
}

fn list2(left: Vec<&'static str>, right: Vec<&'static str>) -> Vec<&'static str> {
    let mut out = vec!["10"];
    out.extend(left);
    out.extend(right);
    out.push("01");
    out
}

fn lambda0_call(body: Vec<&'static str>) -> Vec<&'static str> {
    // (LAMBDA () body) where the empty parameter list is structural D2 framing,
    // not the D3 NIL resident used as a value expression.
    let mut out = vec!["10", "0010", "10", "01"];
    out.extend(body);
    out.push("01");
    out
}

fn count_table() -> Result<[u128; COUNT_LIMIT + 1], Frame3MicroEnumerateError> {
    let mut counts = [0u128; COUNT_LIMIT + 1];
    counts[1] = LEAF_COUNT;
    for nodes in 2..=COUNT_LIMIT {
        let mut total = counts[nodes - 1]; // ATOM

        let mut binary_pairs = 0u128;
        for left_nodes in 1..nodes - 1 {
            let right_nodes = nodes - 1 - left_nodes;
            binary_pairs = binary_pairs
                .checked_add(
                    counts[left_nodes]
                        .checked_mul(counts[right_nodes])
                        .ok_or(Frame3MicroEnumerateError::InternalCountOverflow)?,
                )
                .ok_or(Frame3MicroEnumerateError::InternalCountOverflow)?;
        }
        total = total
            .checked_add(
                binary_pairs
                    .checked_mul(3)
                    .ok_or(Frame3MicroEnumerateError::InternalCountOverflow)?,
            )
            .ok_or(Frame3MicroEnumerateError::InternalCountOverflow)?;

        for test_nodes in 1..nodes.saturating_sub(2) {
            for then_nodes in 1..nodes - 1 - test_nodes {
                let else_nodes = nodes - 1 - test_nodes - then_nodes;
                if else_nodes == 0 {
                    continue;
                }
                let block = counts[test_nodes]
                    .checked_mul(counts[then_nodes])
                    .and_then(|value| value.checked_mul(counts[else_nodes]))
                    .ok_or(Frame3MicroEnumerateError::InternalCountOverflow)?;
                total = total
                    .checked_add(block)
                    .ok_or(Frame3MicroEnumerateError::InternalCountOverflow)?;
            }
        }
        counts[nodes] = total;
    }
    Ok(counts)
}

pub fn exact_node_capacity(nodes: usize) -> Result<u128, Frame3MicroEnumerateError> {
    let counts = count_table()?;
    counts
        .get(nodes)
        .copied()
        .filter(|value| *value != 0)
        .ok_or(Frame3MicroEnumerateError::RankOutOfRange)
}

fn cumulative_before(
    nodes: usize,
    counts: &[u128; COUNT_LIMIT + 1],
) -> Result<u128, Frame3MicroEnumerateError> {
    let mut total = 0u128;
    for value in counts.iter().take(nodes).skip(1) {
        total = total
            .checked_add(*value)
            .ok_or(Frame3MicroEnumerateError::InternalCountOverflow)?;
    }
    Ok(total)
}

fn leaf_from_rank(rank: u128) -> Result<Frame3MicroExpr, Frame3MicroEnumerateError> {
    match rank {
        0 => Ok(Frame3MicroExpr::Zero),
        1 => Ok(Frame3MicroExpr::One),
        2 => Ok(Frame3MicroExpr::Nil),
        3 => Ok(Frame3MicroExpr::QuoteZero),
        4 => Ok(Frame3MicroExpr::QuoteOne),
        5 => Ok(Frame3MicroExpr::QuoteNil),
        _ => Err(Frame3MicroEnumerateError::RankOutOfRange),
    }
}

fn leaf_rank(expr: &Frame3MicroExpr) -> Option<u128> {
    match expr {
        Frame3MicroExpr::Zero => Some(0),
        Frame3MicroExpr::One => Some(1),
        Frame3MicroExpr::Nil => Some(2),
        Frame3MicroExpr::QuoteZero => Some(3),
        Frame3MicroExpr::QuoteOne => Some(4),
        Frame3MicroExpr::QuoteNil => Some(5),
        _ => None,
    }
}

fn unrank_exact(
    nodes: usize,
    mut rank: u128,
    counts: &[u128; COUNT_LIMIT + 1],
) -> Result<Frame3MicroExpr, Frame3MicroEnumerateError> {
    if nodes == 1 {
        return leaf_from_rank(rank);
    }
    if nodes > COUNT_LIMIT || rank >= counts[nodes] {
        return Err(Frame3MicroEnumerateError::RankOutOfRange);
    }

    let atom_block = counts[nodes - 1];
    if rank < atom_block {
        return Ok(Frame3MicroExpr::Atom(Box::new(unrank_exact(
            nodes - 1,
            rank,
            counts,
        )?)));
    }
    rank -= atom_block;

    for family in 0..3 {
        for left_nodes in 1..nodes - 1 {
            let right_nodes = nodes - 1 - left_nodes;
            let block = counts[left_nodes]
                .checked_mul(counts[right_nodes])
                .ok_or(Frame3MicroEnumerateError::InternalCountOverflow)?;
            if rank < block {
                let left_rank = rank / counts[right_nodes];
                let right_rank = rank % counts[right_nodes];
                let left = Box::new(unrank_exact(left_nodes, left_rank, counts)?);
                let right = Box::new(unrank_exact(right_nodes, right_rank, counts)?);
                return Ok(match family {
                    0 => Frame3MicroExpr::Cons(left, right),
                    1 => Frame3MicroExpr::CarCons(left, right),
                    _ => Frame3MicroExpr::CdrCons(left, right),
                });
            }
            rank -= block;
        }
    }

    for test_nodes in 1..nodes.saturating_sub(2) {
        for then_nodes in 1..nodes - 1 - test_nodes {
            let else_nodes = nodes - 1 - test_nodes - then_nodes;
            if else_nodes == 0 {
                continue;
            }
            let then_else = counts[then_nodes]
                .checked_mul(counts[else_nodes])
                .ok_or(Frame3MicroEnumerateError::InternalCountOverflow)?;
            let block = counts[test_nodes]
                .checked_mul(then_else)
                .ok_or(Frame3MicroEnumerateError::InternalCountOverflow)?;
            if rank < block {
                let test_rank = rank / then_else;
                let remainder = rank % then_else;
                let then_rank = remainder / counts[else_nodes];
                let else_rank = remainder % counts[else_nodes];
                return Ok(Frame3MicroExpr::IfAtom(
                    Box::new(unrank_exact(test_nodes, test_rank, counts)?),
                    Box::new(unrank_exact(then_nodes, then_rank, counts)?),
                    Box::new(unrank_exact(else_nodes, else_rank, counts)?),
                ));
            }
            rank -= block;
        }
    }

    Err(Frame3MicroEnumerateError::RankOutOfRange)
}

fn rank_exact(
    expr: &Frame3MicroExpr,
    counts: &[u128; COUNT_LIMIT + 1],
) -> Result<u128, Frame3MicroEnumerateError> {
    let nodes = expr.node_count();
    if nodes == 1 {
        return leaf_rank(expr).ok_or(Frame3MicroEnumerateError::RankOutOfRange);
    }
    if nodes > COUNT_LIMIT {
        return Err(Frame3MicroEnumerateError::ProgramOutsideF3M4);
    }

    let atom_block = counts[nodes - 1];
    if let Frame3MicroExpr::Atom(value) = expr {
        return rank_exact(value, counts);
    }
    let mut rank = atom_block;

    let binary_pairs = (1..nodes - 1)
        .map(|left_nodes| {
            counts[left_nodes]
                .checked_mul(counts[nodes - 1 - left_nodes])
                .ok_or(Frame3MicroEnumerateError::InternalCountOverflow)
        })
        .try_fold(0u128, |sum, value| {
            sum.checked_add(value?)
                .ok_or(Frame3MicroEnumerateError::InternalCountOverflow)
        })?;

    let binary = match expr {
        Frame3MicroExpr::Cons(left, right) => Some((0u128, left, right)),
        Frame3MicroExpr::CarCons(left, right) => Some((1u128, left, right)),
        Frame3MicroExpr::CdrCons(left, right) => Some((2u128, left, right)),
        _ => None,
    };
    if let Some((family, left, right)) = binary {
        rank = rank
            .checked_add(
                family
                    .checked_mul(binary_pairs)
                    .ok_or(Frame3MicroEnumerateError::InternalCountOverflow)?,
            )
            .ok_or(Frame3MicroEnumerateError::InternalCountOverflow)?;
        let left_nodes = left.node_count();
        let right_nodes = right.node_count();
        if left_nodes + right_nodes + 1 != nodes {
            return Err(Frame3MicroEnumerateError::RankOutOfRange);
        }
        for earlier_left in 1..left_nodes {
            let earlier_right = nodes - 1 - earlier_left;
            rank = rank
                .checked_add(
                    counts[earlier_left]
                        .checked_mul(counts[earlier_right])
                        .ok_or(Frame3MicroEnumerateError::InternalCountOverflow)?,
                )
                .ok_or(Frame3MicroEnumerateError::InternalCountOverflow)?;
        }
        rank = rank
            .checked_add(
                rank_exact(left, counts)?
                    .checked_mul(counts[right_nodes])
                    .ok_or(Frame3MicroEnumerateError::InternalCountOverflow)?,
            )
            .and_then(|value| value.checked_add(rank_exact(right, counts).ok()?))
            .ok_or(Frame3MicroEnumerateError::InternalCountOverflow)?;
        return Ok(rank);
    }

    rank = rank
        .checked_add(
            binary_pairs
                .checked_mul(3)
                .ok_or(Frame3MicroEnumerateError::InternalCountOverflow)?,
        )
        .ok_or(Frame3MicroEnumerateError::InternalCountOverflow)?;

    if let Frame3MicroExpr::IfAtom(test, then_value, else_value) = expr {
        let test_nodes = test.node_count();
        let then_nodes = then_value.node_count();
        let else_nodes = else_value.node_count();
        if test_nodes + then_nodes + else_nodes + 1 != nodes {
            return Err(Frame3MicroEnumerateError::RankOutOfRange);
        }

        for earlier_test in 1..test_nodes {
            for earlier_then in 1..nodes - 1 - earlier_test {
                let earlier_else = nodes - 1 - earlier_test - earlier_then;
                if earlier_else == 0 {
                    continue;
                }
                rank = rank
                    .checked_add(
                        counts[earlier_test]
                            .checked_mul(counts[earlier_then])
                            .and_then(|value| value.checked_mul(counts[earlier_else]))
                            .ok_or(Frame3MicroEnumerateError::InternalCountOverflow)?,
                    )
                    .ok_or(Frame3MicroEnumerateError::InternalCountOverflow)?;
            }
        }
        for earlier_then in 1..then_nodes {
            let earlier_else = nodes - 1 - test_nodes - earlier_then;
            if earlier_else == 0 {
                continue;
            }
            rank = rank
                .checked_add(
                    counts[test_nodes]
                        .checked_mul(counts[earlier_then])
                        .and_then(|value| value.checked_mul(counts[earlier_else]))
                        .ok_or(Frame3MicroEnumerateError::InternalCountOverflow)?,
                )
                .ok_or(Frame3MicroEnumerateError::InternalCountOverflow)?;
        }

        let then_else = counts[then_nodes]
            .checked_mul(counts[else_nodes])
            .ok_or(Frame3MicroEnumerateError::InternalCountOverflow)?;
        rank = rank
            .checked_add(
                rank_exact(test, counts)?
                    .checked_mul(then_else)
                    .ok_or(Frame3MicroEnumerateError::InternalCountOverflow)?,
            )
            .and_then(|value| {
                value.checked_add(
                    rank_exact(then_value, counts)
                        .ok()?
                        .checked_mul(counts[else_nodes])?,
                )
            })
            .and_then(|value| value.checked_add(rank_exact(else_value, counts).ok()?))
            .ok_or(Frame3MicroEnumerateError::InternalCountOverflow)?;
        return Ok(rank);
    }

    Err(Frame3MicroEnumerateError::RankOutOfRange)
}

fn update_minimum(slot: &mut Option<u128>, candidate: u128) {
    match slot {
        Some(current) if *current <= candidate => {}
        _ => *slot = Some(candidate),
    }
}

/// Small dynamic-programming proof over the 64 possible capability masks.
///
/// Returns the earliest global rank whose decoded program contains every bit
/// requested by `required`. This searches the grammar algebraically rather
/// than enumerating millions of physical payloads.
pub fn first_global_rank_with_capabilities(
    required: u8,
) -> Result<u128, Frame3MicroEnumerateError> {
    if required & !CAP_ALL != 0 {
        return Err(Frame3MicroEnumerateError::RankOutOfRange);
    }

    let counts = count_table()?;
    let mut minima = vec![[None::<u128>; 64]; COUNT_LIMIT + 1];
    // Node-one order: 0, 1, NIL, QUOTE(0), QUOTE(1), QUOTE(NIL).
    minima[1][0] = Some(0);
    minima[1][usize::from(CAP_QUOTE)] = Some(3);

    let mut cumulative = 0u128;
    for nodes in 1..=COUNT_LIMIT {
        if nodes > 1 {
            // ATOM is the first constructor block for an exact node count.
            for child_mask in 0..64 {
                if let Some(child_rank) = minima[nodes - 1][child_mask] {
                    let mask = child_mask | usize::from(CAP_ATOM);
                    update_minimum(&mut minima[nodes][mask], child_rank);
                }
            }

            let mut binary_pairs = 0u128;
            for left_nodes in 1..nodes - 1 {
                let right_nodes = nodes - 1 - left_nodes;
                binary_pairs = binary_pairs
                    .checked_add(
                        counts[left_nodes]
                            .checked_mul(counts[right_nodes])
                            .ok_or(Frame3MicroEnumerateError::InternalCountOverflow)?,
                    )
                    .ok_or(Frame3MicroEnumerateError::InternalCountOverflow)?;
            }

            let families = [
                CAP_CONS,
                CAP_CONS | CAP_CAR,
                CAP_CONS | CAP_CDR,
            ];
            for (family_index, family_mask) in families.into_iter().enumerate() {
                let mut split_offset = 0u128;
                for left_nodes in 1..nodes - 1 {
                    let right_nodes = nodes - 1 - left_nodes;
                    for left_mask in 0..64 {
                        let Some(left_rank) = minima[left_nodes][left_mask] else {
                            continue;
                        };
                        for right_mask in 0..64 {
                            let Some(right_rank) = minima[right_nodes][right_mask] else {
                                continue;
                            };
                            let rank = counts[nodes - 1]
                                .checked_add(
                                    (family_index as u128)
                                        .checked_mul(binary_pairs)
                                        .ok_or(
                                            Frame3MicroEnumerateError::InternalCountOverflow,
                                        )?,
                                )
                                .and_then(|value| value.checked_add(split_offset))
                                .and_then(|value| {
                                    value.checked_add(
                                        left_rank.checked_mul(counts[right_nodes])?,
                                    )
                                })
                                .and_then(|value| value.checked_add(right_rank))
                                .ok_or(Frame3MicroEnumerateError::InternalCountOverflow)?;
                            let mask = usize::from(family_mask) | left_mask | right_mask;
                            update_minimum(&mut minima[nodes][mask], rank);
                        }
                    }
                    split_offset = split_offset
                        .checked_add(
                            counts[left_nodes]
                                .checked_mul(counts[right_nodes])
                                .ok_or(Frame3MicroEnumerateError::InternalCountOverflow)?,
                        )
                        .ok_or(Frame3MicroEnumerateError::InternalCountOverflow)?;
                }
            }

            let if_base = counts[nodes - 1]
                .checked_add(
                    binary_pairs
                        .checked_mul(3)
                        .ok_or(Frame3MicroEnumerateError::InternalCountOverflow)?,
                )
                .ok_or(Frame3MicroEnumerateError::InternalCountOverflow)?;
            let mut split_offset = 0u128;
            for test_nodes in 1..nodes.saturating_sub(2) {
                for then_nodes in 1..nodes - 1 - test_nodes {
                    let else_nodes = nodes - 1 - test_nodes - then_nodes;
                    if else_nodes == 0 {
                        continue;
                    }
                    let then_else = counts[then_nodes]
                        .checked_mul(counts[else_nodes])
                        .ok_or(Frame3MicroEnumerateError::InternalCountOverflow)?;
                    for test_mask in 0..64 {
                        let Some(test_rank) = minima[test_nodes][test_mask] else {
                            continue;
                        };
                        for then_mask in 0..64 {
                            let Some(then_rank) = minima[then_nodes][then_mask] else {
                                continue;
                            };
                            for else_mask in 0..64 {
                                let Some(else_rank) = minima[else_nodes][else_mask] else {
                                    continue;
                                };
                                let rank = if_base
                                    .checked_add(split_offset)
                                    .and_then(|value| {
                                        value.checked_add(test_rank.checked_mul(then_else)?)
                                    })
                                    .and_then(|value| {
                                        value.checked_add(
                                            then_rank.checked_mul(counts[else_nodes])?,
                                        )
                                    })
                                    .and_then(|value| value.checked_add(else_rank))
                                    .ok_or(
                                        Frame3MicroEnumerateError::InternalCountOverflow,
                                    )?;
                                let mask = usize::from(CAP_COND | CAP_ATOM)
                                    | test_mask
                                    | then_mask
                                    | else_mask;
                                update_minimum(&mut minima[nodes][mask], rank);
                            }
                        }
                    }
                    split_offset = split_offset
                        .checked_add(
                            counts[test_nodes]
                                .checked_mul(counts[then_nodes])
                                .and_then(|value| value.checked_mul(counts[else_nodes]))
                                .ok_or(Frame3MicroEnumerateError::InternalCountOverflow)?,
                        )
                        .ok_or(Frame3MicroEnumerateError::InternalCountOverflow)?;
                }
            }
        }

        let mut earliest = None;
        for mask in 0..64 {
            if (mask as u8) & required == required {
                if let Some(rank) = minima[nodes][mask] {
                    update_minimum(&mut earliest, rank);
                }
            }
        }
        if let Some(exact_rank) = earliest {
            let global = cumulative
                .checked_add(exact_rank)
                .ok_or(Frame3MicroEnumerateError::InternalCountOverflow)?;
            if global < F3M_END {
                return Ok(global);
            }
            return Err(Frame3MicroEnumerateError::ProgramOutsideF3M4);
        }

        cumulative = cumulative
            .checked_add(counts[nodes])
            .ok_or(Frame3MicroEnumerateError::InternalCountOverflow)?;
    }

    Err(Frame3MicroEnumerateError::RankOutOfRange)
}

pub fn unrank_global(global_rank: u128) -> Result<Frame3MicroExpr, Frame3MicroEnumerateError> {
    if global_rank >= F3M_END {
        return Err(Frame3MicroEnumerateError::ProgramOutsideF3M4);
    }
    let counts = count_table()?;
    let mut remaining = global_rank;
    for nodes in 1..=COUNT_LIMIT {
        if remaining < counts[nodes] {
            return unrank_exact(nodes, remaining, &counts);
        }
        remaining -= counts[nodes];
    }
    Err(Frame3MicroEnumerateError::RankOutOfRange)
}

pub fn rank_global(expr: &Frame3MicroExpr) -> Result<u128, Frame3MicroEnumerateError> {
    let counts = count_table()?;
    let nodes = expr.node_count();
    if nodes > COUNT_LIMIT {
        return Err(Frame3MicroEnumerateError::ProgramOutsideF3M4);
    }
    let rank = cumulative_before(nodes, &counts)?
        .checked_add(rank_exact(expr, &counts)?)
        .ok_or(Frame3MicroEnumerateError::InternalCountOverflow)?;
    if rank >= F3M_END {
        return Err(Frame3MicroEnumerateError::ProgramOutsideF3M4);
    }
    Ok(rank)
}

fn profile_offset(profile: Frame3MicroProfile) -> u128 {
    match profile {
        Frame3MicroProfile::F3M1 => 0,
        Frame3MicroProfile::F3M2 => F3M2_OFFSET,
        Frame3MicroProfile::F3M3 => F3M3_OFFSET,
        Frame3MicroProfile::F3M4 => F3M4_OFFSET,
        // Logical profile ordering only. F3M-5 uses its own prefix-stable
        // extension ranker rather than extending the legacy grammar order.
        Frame3MicroProfile::F3M5 => F3M_END,
    }
}

fn profile_capacity(profile: Frame3MicroProfile) -> u128 {
    match profile {
        Frame3MicroProfile::F3M1 => F3M1_CAPACITY,
        Frame3MicroProfile::F3M2 => F3M2_CAPACITY,
        Frame3MicroProfile::F3M3 => F3M3_CAPACITY,
        Frame3MicroProfile::F3M4 => F3M4_CAPACITY,
        Frame3MicroProfile::F3M5 => F3M5_CAPACITY,
    }
}

pub fn decode_payload(payload: &[u8]) -> Result<Frame3MicroExpr, Frame3MicroEnumerateError> {
    let profile = frame3_micro_profile(payload)
        .map_err(|error| Frame3MicroEnumerateError::UnsupportedLength(error.physical_bytes))?;
    let mut local = 0u128;
    for byte in payload {
        local = (local << 8) | u128::from(*byte);
    }
    if local >= profile_capacity(profile) {
        return Err(Frame3MicroEnumerateError::RankOutOfRange);
    }
    if profile == Frame3MicroProfile::F3M5 {
        return crate::frame3_micro_v5::decode_extension(local);
    }
    unrank_global(profile_offset(profile) + local)
}

pub fn encode_expr(expr: &Frame3MicroExpr) -> Result<Vec<u8>, Frame3MicroEnumerateError> {
    let (profile, local) = if crate::frame3_micro_v5::contains_extension(expr) {
        (
            Frame3MicroProfile::F3M5,
            crate::frame3_micro_v5::encode_extension(expr)?,
        )
    } else {
        let global = rank_global(expr)?;
        if global < F3M1_CAPACITY {
            (Frame3MicroProfile::F3M1, global)
        } else if global < F3M3_OFFSET {
            (Frame3MicroProfile::F3M2, global - F3M2_OFFSET)
        } else if global < F3M4_OFFSET {
            (Frame3MicroProfile::F3M3, global - F3M3_OFFSET)
        } else if global < F3M_END {
            (Frame3MicroProfile::F3M4, global - F3M4_OFFSET)
        } else {
            return Err(Frame3MicroEnumerateError::ProgramOutsideF3M4);
        }
    };

    let width = profile.physical_bytes();
    let mut payload = vec![0u8; width];
    let mut value = local;
    for byte in payload.iter_mut().rev() {
        *byte = value as u8;
        value >>= 8;
    }
    if value != 0 {
        return Err(Frame3MicroEnumerateError::RankOutOfRange);
    }
    Ok(payload)
}

pub const fn profile_program_capacity(profile: Frame3MicroProfile) -> u128 {
    match profile {
        Frame3MicroProfile::F3M1 => F3M1_CAPACITY,
        Frame3MicroProfile::F3M2 => F3M2_CAPACITY,
        Frame3MicroProfile::F3M3 => F3M3_CAPACITY,
        Frame3MicroProfile::F3M4 => F3M4_CAPACITY,
        Frame3MicroProfile::F3M5 => F3M5_CAPACITY,
    }
}

pub const fn profile_global_offset(profile: Frame3MicroProfile) -> u128 {
    match profile {
        Frame3MicroProfile::F3M1 => 0,
        Frame3MicroProfile::F3M2 => F3M2_OFFSET,
        Frame3MicroProfile::F3M3 => F3M3_OFFSET,
        Frame3MicroProfile::F3M4 => F3M4_OFFSET,
        Frame3MicroProfile::F3M5 => F3M_END,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{eval_parsed_expressions, parse_canonical_binary, Session};
    use std::collections::HashSet;

    #[test]
    fn capacities_use_every_payload_code() {
        assert_eq!(profile_program_capacity(Frame3MicroProfile::F3M1), 256);
        assert_eq!(profile_program_capacity(Frame3MicroProfile::F3M2), 65_536);
        assert_eq!(
            profile_program_capacity(Frame3MicroProfile::F3M3),
            16_777_216
        );
        assert_eq!(
            profile_program_capacity(Frame3MicroProfile::F3M4),
            4_294_967_296
        );
        assert_eq!(
            profile_program_capacity(Frame3MicroProfile::F3M5),
            1_099_511_627_776
        );
        assert_eq!(profile_global_offset(Frame3MicroProfile::F3M2), 256);
        assert_eq!(profile_global_offset(Frame3MicroProfile::F3M3), 65_792);
        assert_eq!(
            profile_global_offset(Frame3MicroProfile::F3M4),
            16_843_008
        );
    }

    #[test]
    fn grammar_has_enough_distinct_programs_for_all_four_profiles() {
        let counts = count_table().unwrap();
        let through_eight = (1..=8).map(|nodes| counts[nodes]).sum::<u128>();
        let through_nine = through_eight + counts[9];
        let through_eleven = (1..=11).map(|nodes| counts[nodes]).sum::<u128>();
        let through_twelve = through_eleven + counts[12];
        assert_eq!(through_eight, 3_839_232);
        assert_eq!(counts[9], 31_422_390);
        assert_eq!(through_nine, 35_261_622);
        assert_eq!(through_eleven, 3_149_929_038);
        assert_eq!(counts[12], 27_233_592_018);
        // F3M-4 begins inside the 9-node layer and extends into 12-node forms.
        assert!(through_eight < F3M4_OFFSET);
        assert!(through_nine > F3M4_OFFSET);
        assert!(through_eleven < F3M_END);
        assert!(through_twelve >= F3M_END);
    }

    #[test]
    fn one_byte_space_is_full_unique_and_canonical() {
        let mut seen = HashSet::new();
        for value in 0u16..=255 {
            let payload = [value as u8];
            let expr = decode_payload(&payload).unwrap();
            assert!(seen.insert(expr.clone()), "duplicate one-byte program");
            assert_eq!(encode_expr(&expr).unwrap(), payload);
        }
        assert_eq!(seen.len(), 256);
    }

    #[test]
    fn census_one_byte_cool_programs() {
        let mut best_overall: Option<
            ((u32, usize, usize, usize, usize, usize, u32), Frame3MicroExpr),
        > = None;
        let mut deepest: Option<
            ((usize, usize, u32, usize, usize, u32), Frame3MicroExpr),
        > = None;
        let mut best_car: Option<
            ((u32, usize, usize, usize, usize, usize, u32), Frame3MicroExpr),
        > = None;
        let mut best_cdr: Option<
            ((u32, usize, usize, usize, usize, usize, u32), Frame3MicroExpr),
        > = None;
        let mut best_cons: Option<
            ((u32, usize, usize, usize, usize, usize, u32), Frame3MicroExpr),
        > = None;
        let mut conditional_count_total = 0usize;

        for local in 0u32..=u8::MAX as u32 {
            let payload = [local as u8];
            let expr = decode_payload(&payload).unwrap();

            let cap = expr.capability_mask().count_ones();
            let cond = conditional_count(&expr);
            let ops = encoded_operation_count(&expr);
            let depth = expression_depth(&expr);
            let words = expr.exact_words().len();
            let nodes = expr.node_count();
            let expressive = (cap, cond, ops, depth, words, nodes, local);

            if best_overall
                .as_ref()
                .is_none_or(|(current, _)| expressive > *current)
            {
                best_overall = Some((expressive, expr.clone()));
            }
            if cond > 0 {
                conditional_count_total += 1;
            }
            if expr.capability_mask() & CAP_CAR != 0
                && best_car
                    .as_ref()
                    .is_none_or(|(current, _)| expressive > *current)
            {
                best_car = Some((expressive, expr.clone()));
            }
            if expr.capability_mask() & CAP_CDR != 0
                && best_cdr
                    .as_ref()
                    .is_none_or(|(current, _)| expressive > *current)
            {
                best_cdr = Some((expressive, expr.clone()));
            }
            if expr.capability_mask() & CAP_CONS != 0
                && best_cons
                    .as_ref()
                    .is_none_or(|(current, _)| expressive > *current)
            {
                best_cons = Some((expressive, expr.clone()));
            }

            let depth_score = (depth, ops, cap, words, nodes, local);
            if deepest
                .as_ref()
                .is_none_or(|(current, _)| depth_score > *current)
            {
                deepest = Some((depth_score, expr));
            }
        }

        let (best_score, best_expr) = best_overall.unwrap();
        eprintln!(
            "F3M1_BEST_OVERALL={:02X}\tcap={}\tcond={}\tops={}\tdepth={}\twords={}\tnodes={}\t{}",
            best_score.6,
            best_score.0,
            best_score.1,
            best_score.2,
            best_score.3,
            best_score.4,
            best_score.5,
            best_expr.human_projection()
        );

        let (depth_score, depth_expr) = deepest.unwrap();
        eprintln!(
            "F3M1_DEEPEST={:02X}\tdepth={}\tops={}\tcap={}\twords={}\tnodes={}\t{}",
            depth_score.5,
            depth_score.0,
            depth_score.1,
            depth_score.2,
            depth_score.3,
            depth_score.4,
            depth_expr.human_projection()
        );

        for (label, candidate) in [
            ("F3M1_BEST_CAR", best_car.unwrap()),
            ("F3M1_BEST_CDR", best_cdr.unwrap()),
            ("F3M1_BEST_CONS", best_cons.unwrap()),
        ] {
            let (score, expr) = candidate;
            eprintln!(
                "{label}={:02X}\tcap={}\tcond={}\tops={}\tdepth={}\twords={}\tnodes={}\t{}",
                score.6,
                score.0,
                score.1,
                score.2,
                score.3,
                score.4,
                score.5,
                expr.human_projection()
            );
        }

        eprintln!("F3M1_COND_CODES={conditional_count_total}");

        assert_eq!(conditional_count_total, 0);

        for (code, projection) in [
            (0x1a, "CONS(1,NIL)"),
            (0x3e, "CAR(CONS(1,NIL))"),
            (0x67, "CDR(CONS(NIL,1))"),
            (0x83, "ATOM(ATOM(ATOM(QUOTE(NIL))))"),
            (0xcb, "ATOM(CAR(CONS(QUOTE(NIL),QUOTE(NIL))))"),
            (0xef, "ATOM(CDR(CONS(QUOTE(NIL),QUOTE(NIL))))"),
        ] {
            let expr = decode_payload(&[code]).unwrap();
            assert_eq!(expr.human_projection(), projection);
            assert_eq!(encode_expr(&expr).unwrap(), vec![code]);
        }

        for code in [0x3e, 0x67, 0x83, 0xcb, 0xef] {
            let expr = decode_payload(&[code]).unwrap();
            let source = expr.exact_words().join(" ");
            let actual = eval_parsed_expressions(
                &parse_canonical_binary(&source).unwrap(),
                &mut Session::default(),
            )
            .unwrap();
            let expected = eval_parsed_expressions(
                &parse_canonical_binary("1").unwrap(),
                &mut Session::default(),
            )
            .unwrap();
            assert_eq!(actual.value, expected.value);
            assert_eq!(actual.output, expected.output);
        }
        for local in 0u32..=u8::MAX as u32 {
            let expr = decode_payload(&[local as u8]).unwrap();
            let source = expr.exact_words().join(" ");
            let parsed = parse_canonical_binary(&source).expect("F3M-1 census source");
            eval_parsed_expressions(&parsed, &mut Session::default())
                .expect("every F3M-1 census candidate must execute");
        }
    }

    #[test]
    fn two_byte_space_roundtrips_exhaustively() {
        for value in 0u32..=u16::MAX as u32 {
            let payload = (value as u16).to_be_bytes();
            let expr = decode_payload(&payload).unwrap();
            assert_eq!(encode_expr(&expr).unwrap(), payload);
        }
    }

    #[test]
    fn census_two_byte_cool_programs() {
        let mut best_overall: Option<
            ((u32, usize, usize, usize, usize, usize, u32), Frame3MicroExpr),
        > = None;
        let mut best_conditional: Option<
            ((u32, usize, usize, usize, usize, usize, u32), Frame3MicroExpr),
        > = None;
        let mut deepest: Option<
            ((usize, usize, u32, usize, usize, u32), Frame3MicroExpr),
        > = None;
        let mut most_words: Option<
            ((usize, usize, u32, usize, usize, u32), Frame3MicroExpr),
        > = None;
        let mut first_conditional: Option<(u32, Frame3MicroExpr)> = None;
        let mut best_selector_pair: Option<
            ((u32, usize, usize, usize, usize, usize, u32), Frame3MicroExpr),
        > = None;

        for local in 0u32..=u16::MAX as u32 {
            let payload = (local as u16).to_be_bytes();
            let expr = decode_payload(&payload).unwrap();

            let cap = expr.capability_mask().count_ones();
            let cond = conditional_count(&expr);
            let ops = encoded_operation_count(&expr);
            let depth = expression_depth(&expr);
            let words = expr.exact_words().len();
            let nodes = expr.node_count();

            let expressive = (cap, cond, ops, depth, words, nodes, local);
            if best_overall
                .as_ref()
                .is_none_or(|(current, _)| expressive > *current)
            {
                best_overall = Some((expressive, expr.clone()));
            }
            if cond > 0
                && best_conditional
                    .as_ref()
                    .is_none_or(|(current, _)| expressive > *current)
            {
                best_conditional = Some((expressive, expr.clone()));
            }
            if cond > 0 && first_conditional.is_none() {
                first_conditional = Some((local, expr.clone()));
            }

            let selector_mask = CAP_CAR | CAP_CDR | CAP_CONS;
            if expr.capability_mask() & selector_mask == selector_mask
                && best_selector_pair
                    .as_ref()
                    .is_none_or(|(current, _)| expressive > *current)
            {
                best_selector_pair = Some((expressive, expr.clone()));
            }

            let depth_score = (depth, ops, cap, words, nodes, local);
            if deepest
                .as_ref()
                .is_none_or(|(current, _)| depth_score > *current)
            {
                deepest = Some((depth_score, expr.clone()));
            }

            let word_score = (words, ops, cap, depth, nodes, local);
            if most_words
                .as_ref()
                .is_none_or(|(current, _)| word_score > *current)
            {
                most_words = Some((word_score, expr));
            }
        }

        let (best_score, best_expr) = best_overall.unwrap();
        eprintln!(
            "F3M2_BEST_OVERALL={:04X}\tcap={}\tcond={}\tops={}\tdepth={}\twords={}\tnodes={}\t{}",
            best_score.6,
            best_score.0,
            best_score.1,
            best_score.2,
            best_score.3,
            best_score.4,
            best_score.5,
            best_expr.human_projection()
        );

        let (cond_score, cond_expr) = best_conditional.unwrap();
        eprintln!(
            "F3M2_BEST_COND={:04X}\tcap={}\tcond={}\tops={}\tdepth={}\twords={}\tnodes={}\t{}",
            cond_score.6,
            cond_score.0,
            cond_score.1,
            cond_score.2,
            cond_score.3,
            cond_score.4,
            cond_score.5,
            cond_expr.human_projection()
        );

        let (first_cond_code, first_cond_expr) = first_conditional.unwrap();
        eprintln!(
            "F3M2_FIRST_COND={:04X}\tcap={}\tops={}\tdepth={}\twords={}\tnodes={}\t{}",
            first_cond_code,
            first_cond_expr.capability_mask().count_ones(),
            encoded_operation_count(&first_cond_expr),
            expression_depth(&first_cond_expr),
            first_cond_expr.exact_words().len(),
            first_cond_expr.node_count(),
            first_cond_expr.human_projection()
        );

        let (selector_score, selector_expr) = best_selector_pair.unwrap();
        eprintln!(
            "F3M2_BEST_CAR_CDR={:04X}\tcap={}\tcond={}\tops={}\tdepth={}\twords={}\tnodes={}\t{}",
            selector_score.6,
            selector_score.0,
            selector_score.1,
            selector_score.2,
            selector_score.3,
            selector_score.4,
            selector_score.5,
            selector_expr.human_projection()
        );

        let (depth_score, depth_expr) = deepest.unwrap();
        eprintln!(
            "F3M2_DEEPEST={:04X}\tdepth={}\tops={}\tcap={}\twords={}\tnodes={}\t{}",
            depth_score.5,
            depth_score.0,
            depth_score.1,
            depth_score.2,
            depth_score.3,
            depth_score.4,
            depth_expr.human_projection()
        );

        let (word_score, word_expr) = most_words.unwrap();
        eprintln!(
            "F3M2_MOST_WORDS={:04X}\twords={}\tops={}\tcap={}\tdepth={}\tnodes={}\t{}",
            word_score.5,
            word_score.0,
            word_score.1,
            word_score.2,
            word_score.3,
            word_score.4,
            word_expr.human_projection()
        );

        assert_eq!(best_score.6, 0xfc25);
        assert_eq!(best_score.0, 5);
        assert_eq!(best_score.1, 1);
        assert_eq!(
            best_expr.human_projection(),
            "ATOM(CAR(CONS(QUOTE(NIL),IF-ATOM(QUOTE(NIL),QUOTE(NIL),QUOTE(NIL)))))"
        );

        assert_eq!(first_cond_code, 0x00c8);
        assert_eq!(
            first_cond_expr.human_projection(),
            "IF-ATOM(0,0,0)"
        );

        assert_eq!(selector_score.6, 0xfb4d);
        assert_eq!(
            selector_expr.human_projection(),
            "ATOM(CAR(CONS(QUOTE(NIL),CDR(CONS(ATOM(QUOTE(NIL)),QUOTE(NIL))))))"
        );

        assert_eq!(depth_score.5, 0xbb51);
        assert_eq!(depth_score.0, 7);
        assert_eq!(
            depth_expr.human_projection(),
            "ATOM(ATOM(ATOM(ATOM(ATOM(ATOM(QUOTE(NIL)))))))"
        );

        for (score, expr, expected_source) in [
            (best_score.6, best_expr, "1"),
            (cond_score.6, cond_expr, "1"),
            (first_cond_code, first_cond_expr, "0"),
            (selector_score.6, selector_expr, "1"),
            (depth_score.5, depth_expr, "1"),
            (word_score.5, word_expr, "1"),
        ] {
            let payload = (score as u16).to_be_bytes();
            assert_eq!(encode_expr(&expr).unwrap(), payload);
            let source = expr.exact_words().join(" ");
            let parsed = parse_canonical_binary(&source).expect("F3M-2 candidate source");
            let actual = eval_parsed_expressions(&parsed, &mut Session::default())
                .expect("F3M-2 candidate must execute");
            let expected = eval_parsed_expressions(
                &parse_canonical_binary(expected_source).unwrap(),
                &mut Session::default(),
            )
            .unwrap();
            assert_eq!(actual.value, expected.value);
            assert_eq!(actual.output, expected.output);
        }
    }

    #[test]
    fn three_byte_boundaries_and_samples_roundtrip() {
        for value in [0u32, 1, 0x12_34_56, 0x2f_18_59, 0x80_00_00, 0xff_ff_ff] {
            let payload = [
                ((value >> 16) & 0xff) as u8,
                ((value >> 8) & 0xff) as u8,
                (value & 0xff) as u8,
            ];
            let expr = decode_payload(&payload).unwrap();
            assert_eq!(encode_expr(&expr).unwrap(), payload);
        }
    }

    #[test]
    fn five_byte_boundaries_and_stratified_samples_roundtrip() {
        for value in [
            0u64,
            1,
            0x00_12_34_56_78,
            0x80_00_00_00_00,
            0xde_ad_be_ef_01,
            0xff_ff_ff_ff_ff,
        ] {
            let payload = [
                ((value >> 32) & 0xff) as u8,
                ((value >> 24) & 0xff) as u8,
                ((value >> 16) & 0xff) as u8,
                ((value >> 8) & 0xff) as u8,
                (value & 0xff) as u8,
            ];
            let expr = decode_payload(&payload).unwrap();
            assert!(crate::frame3_micro_v5::contains_extension(&expr));
            assert_eq!(encode_expr(&expr).unwrap(), payload);
        }

        // One deterministic point from every high-16 bucket. This spans the
        // full 40-bit interval without pretending to exhaust 2^40 payloads.
        for high in 0u64..=u16::MAX as u64 {
            let low24 = ((high * 0x9e37) ^ 0x5a_a5_3c) & 0xff_ff_ff;
            let value = (high << 24) | low24;
            let payload = [
                ((value >> 32) & 0xff) as u8,
                ((value >> 24) & 0xff) as u8,
                ((value >> 16) & 0xff) as u8,
                ((value >> 8) & 0xff) as u8,
                (value & 0xff) as u8,
            ];
            let expr = decode_payload(&payload).unwrap();
            assert_eq!(encode_expr(&expr).unwrap(), payload);
        }
    }

    #[test]
    fn five_byte_profile_adds_missing_d3_eq_and_d4_lambda_without_remapping_old_bytes() {
        let lambda_payload = [0u8; 5];
        let lambda = decode_payload(&lambda_payload).unwrap();
        assert_eq!(
            lambda,
            Frame3MicroExpr::Lambda0(Box::new(Frame3MicroExpr::Zero))
        );
        assert_eq!(encode_expr(&lambda).unwrap(), lambda_payload);
        assert_eq!(lambda.capability_mask() & CAP_LAMBDA, CAP_LAMBDA);

        let eq = Frame3MicroExpr::EqAtom(
            Box::new(Frame3MicroExpr::Zero),
            Box::new(Frame3MicroExpr::One),
        );
        let eq_payload = encode_expr(&eq).unwrap();
        assert_eq!(eq_payload.len(), 5);
        assert_eq!(decode_payload(&eq_payload).unwrap(), eq);
        assert_eq!(eq.capability_mask() & CAP_EQ, CAP_EQ);

        // Frozen trophies from old profiles remain byte-identical.
        for payload in [
            vec![0x3e],
            vec![0xfc, 0x25],
            vec![0x2f, 0x18, 0x59],
            vec![0xde, 0xad, 0xbe, 0xef],
        ] {
            let expr = decode_payload(&payload).unwrap();
            assert_eq!(encode_expr(&expr).unwrap(), payload);
        }
    }

    #[test]
    fn five_byte_single_program_carries_all_d1_d3_capabilities_plus_lambda() {
        let expr = Frame3MicroExpr::IfAtom(
            Box::new(Frame3MicroExpr::EqAtom(
                Box::new(Frame3MicroExpr::Lambda0(Box::new(
                    Frame3MicroExpr::QuoteZero,
                ))),
                Box::new(Frame3MicroExpr::QuoteOne),
            )),
            Box::new(Frame3MicroExpr::CarCons(
                Box::new(Frame3MicroExpr::QuoteZero),
                Box::new(Frame3MicroExpr::CdrCons(
                    Box::new(Frame3MicroExpr::QuoteOne),
                    Box::new(Frame3MicroExpr::QuoteNil),
                )),
            )),
            Box::new(Frame3MicroExpr::QuoteNil),
        );

        assert_eq!(expr.capability_mask(), CAP_ALL_F3M5);
        let payload = encode_expr(&expr).expect("all-basis F3M5 program must fit five bytes");
        assert_eq!(payload.len(), 5);
        assert_eq!(decode_payload(&payload).unwrap(), expr);

        let source = expr.exact_words().join(" ");
        let actual = eval_parsed_expressions(
            &parse_canonical_binary(&source).expect("all-basis F3M5 source"),
            &mut Session::default(),
        )
        .expect("all-basis F3M5 program must execute");
        assert_eq!(actual.value.as_predicate_bit(), Some(false));

        eprintln!(
            "F3M5_ALL_D1_D3_LAMBDA={}\tnodes={}\twords={}\t{}",
            payload.iter().map(|byte| format!("{byte:02X}")).collect::<String>(),
            expr.node_count(),
            expr.exact_words().len(),
            expr.human_projection()
        );
    }

    #[test]
    fn five_byte_eq_program_executes_and_lambda_decodes_to_a_real_closure() {
        let eq = Frame3MicroExpr::EqAtom(
            Box::new(Frame3MicroExpr::Zero),
            Box::new(Frame3MicroExpr::One),
        );
        let eq_source = eq.exact_words().join(" ");
        let eq_result = eval_parsed_expressions(
            &parse_canonical_binary(&eq_source).expect("F3M5 EQ source"),
            &mut Session::default(),
        )
        .expect("F3M5 EQ must execute");
        assert_eq!(eq_result.value.as_predicate_bit(), Some(true));

        let lambda = Frame3MicroExpr::Lambda0(Box::new(Frame3MicroExpr::QuoteOne));
        let lambda_source = lambda.exact_words().join(" ");
        let lambda_result = eval_parsed_expressions(
            &parse_canonical_binary(&lambda_source).expect("F3M5 LAMBDA source"),
            &mut Session::default(),
        )
        .expect("F3M5 LAMBDA must evaluate");
        assert!(matches!(lambda_result.value, crate::Value::Closure(_)));
    }

    #[test]
    fn four_byte_boundaries_and_stratified_samples_roundtrip() {
        for value in [
            0u32,
            1,
            0x12_34_56_78,
            0x80_00_00_00,
            0xde_ad_be_ef,
            0xff_ff_ff_ff,
        ] {
            let payload = value.to_be_bytes();
            let expr = decode_payload(&payload).unwrap();
            assert_eq!(encode_expr(&expr).unwrap(), payload);
        }

        // Deterministic coverage across the full 32-bit local-rank interval:
        // one point from every high-16 bucket, without claiming exhaustive 2^32.
        for high in 0u32..=u16::MAX as u32 {
            let low = (high ^ 0xa5a5) & 0xffff;
            let value = (high << 16) | low;
            let payload = value.to_be_bytes();
            let expr = decode_payload(&payload).unwrap();
            assert_eq!(encode_expr(&expr).unwrap(), payload);
        }
    }

    #[test]
    fn four_byte_deadbeef_is_a_real_executable_program() {
        let payload = [0xde, 0xad, 0xbe, 0xef];
        let expr = decode_payload(&payload).unwrap();
        assert_eq!(
            expr.human_projection(),
            "ATOM(CONS(ATOM(CDR(CONS(QUOTE(0),CAR(CONS(NIL,QUOTE(0)))))),ATOM(ATOM(ATOM(QUOTE(NIL))))))"
        );
        assert_eq!(expr.node_count(), 12);
        assert_eq!(encode_expr(&expr).unwrap(), payload);

        let source = expr.exact_words().join(" ");
        let parsed = parse_canonical_binary(&source).expect("F3M-4 DEADBEEF source");
        let actual = eval_parsed_expressions(&parsed, &mut Session::default())
            .expect("F3M-4 DEADBEEF witness executes");
        let expected = eval_parsed_expressions(
            &parse_canonical_binary("0").unwrap(),
            &mut Session::default(),
        )
        .unwrap();
        assert_eq!(actual.value, expected.value);
        assert_eq!(actual.output, expected.output);

        eprintln!(
            "F3M4_DEADBEEF={}\tnodes={}\twords={}\t{}",
            payload.iter().map(|byte| format!("{byte:02x}")).collect::<String>(),
            expr.node_count(),
            expr.exact_words().len(),
            expr.human_projection()
        );
    }

    fn encoded_operation_count(expr: &Frame3MicroExpr) -> usize {
        let own = match expr {
            Frame3MicroExpr::Zero | Frame3MicroExpr::One | Frame3MicroExpr::Nil => 0,
            Frame3MicroExpr::QuoteZero
            | Frame3MicroExpr::QuoteOne
            | Frame3MicroExpr::QuoteNil => 1,
            Frame3MicroExpr::Atom(_)
            | Frame3MicroExpr::Cons(_, _)
            | Frame3MicroExpr::Lambda0(_) => 1,
            Frame3MicroExpr::CarCons(_, _)
            | Frame3MicroExpr::CdrCons(_, _)
            | Frame3MicroExpr::IfAtom(_, _, _)
            | Frame3MicroExpr::EqAtom(_, _) => 2,
        };
        own + match expr {
            Frame3MicroExpr::Atom(value) | Frame3MicroExpr::Lambda0(value) => {
                encoded_operation_count(value)
            }
            Frame3MicroExpr::Cons(left, right)
            | Frame3MicroExpr::CarCons(left, right)
            | Frame3MicroExpr::CdrCons(left, right)
            | Frame3MicroExpr::EqAtom(left, right) => {
                encoded_operation_count(left) + encoded_operation_count(right)
            }
            Frame3MicroExpr::IfAtom(test, then_value, else_value) => {
                encoded_operation_count(test)
                    + encoded_operation_count(then_value)
                    + encoded_operation_count(else_value)
            }
            _ => 0,
        }
    }

    fn conditional_count(expr: &Frame3MicroExpr) -> usize {
        let own = usize::from(matches!(expr, Frame3MicroExpr::IfAtom(_, _, _)));
        own + match expr {
            Frame3MicroExpr::Atom(value) | Frame3MicroExpr::Lambda0(value) => {
                conditional_count(value)
            }
            Frame3MicroExpr::Cons(left, right)
            | Frame3MicroExpr::CarCons(left, right)
            | Frame3MicroExpr::CdrCons(left, right)
            | Frame3MicroExpr::EqAtom(left, right) => {
                conditional_count(left) + conditional_count(right)
            }
            Frame3MicroExpr::IfAtom(test, then_value, else_value) => {
                conditional_count(test)
                    + conditional_count(then_value)
                    + conditional_count(else_value)
            }
            _ => 0,
        }
    }

    fn expression_depth(expr: &Frame3MicroExpr) -> usize {
        1 + match expr {
            Frame3MicroExpr::Atom(value) | Frame3MicroExpr::Lambda0(value) => {
                expression_depth(value)
            }
            Frame3MicroExpr::Cons(left, right)
            | Frame3MicroExpr::CarCons(left, right)
            | Frame3MicroExpr::CdrCons(left, right)
            | Frame3MicroExpr::EqAtom(left, right) => {
                expression_depth(left).max(expression_depth(right))
            }
            Frame3MicroExpr::IfAtom(test, then_value, else_value) => expression_depth(test)
                .max(expression_depth(then_value))
                .max(expression_depth(else_value)),
            _ => 0,
        }
    }

    fn earliest_expr_with_conditionals(
        max_nodes: usize,
        target_conditionals: usize,
    ) -> Option<(usize, Frame3MicroExpr)> {
        let counts = count_table().unwrap();
        let mut best = vec![vec![None::<Frame3MicroExpr>; target_conditionals + 1]; max_nodes + 1];
        best[1][0] = Some(Frame3MicroExpr::Zero);

        let consider = |slot: &mut Option<Frame3MicroExpr>, candidate: Frame3MicroExpr| {
            let candidate_rank = rank_exact(&candidate, &counts).unwrap();
            if slot
                .as_ref()
                .is_none_or(|current| candidate_rank < rank_exact(current, &counts).unwrap())
            {
                *slot = Some(candidate);
            }
        };

        for nodes in 2..=max_nodes {
            for conds in 0..=target_conditionals {
                if let Some(child) = best[nodes - 1][conds].clone() {
                    consider(
                        &mut best[nodes][conds],
                        Frame3MicroExpr::Atom(Box::new(child)),
                    );
                }

                for left_nodes in 1..nodes - 1 {
                    let right_nodes = nodes - 1 - left_nodes;
                    for left_conds in 0..=conds {
                        let right_conds = conds - left_conds;
                        let (Some(left), Some(right)) = (
                            best[left_nodes][left_conds].clone(),
                            best[right_nodes][right_conds].clone(),
                        ) else {
                            continue;
                        };
                        for family in 0..3 {
                            let candidate = match family {
                                0 => Frame3MicroExpr::Cons(
                                    Box::new(left.clone()),
                                    Box::new(right.clone()),
                                ),
                                1 => Frame3MicroExpr::CarCons(
                                    Box::new(left.clone()),
                                    Box::new(right.clone()),
                                ),
                                _ => Frame3MicroExpr::CdrCons(
                                    Box::new(left.clone()),
                                    Box::new(right.clone()),
                                ),
                            };
                            consider(&mut best[nodes][conds], candidate);
                        }
                    }
                }

                if conds > 0 {
                    for test_nodes in 1..nodes.saturating_sub(2) {
                        for then_nodes in 1..nodes - 1 - test_nodes {
                            let else_nodes = nodes - 1 - test_nodes - then_nodes;
                            if else_nodes == 0 {
                                continue;
                            }
                            for test_conds in 0..conds {
                                for then_conds in 0..conds - test_conds {
                                    let used = 1 + test_conds + then_conds;
                                    if used > conds {
                                        continue;
                                    }
                                    let else_conds = conds - used;
                                    let (Some(test), Some(then_value), Some(else_value)) = (
                                        best[test_nodes][test_conds].clone(),
                                        best[then_nodes][then_conds].clone(),
                                        best[else_nodes][else_conds].clone(),
                                    ) else {
                                        continue;
                                    };
                                    consider(
                                        &mut best[nodes][conds],
                                        Frame3MicroExpr::IfAtom(
                                            Box::new(test),
                                            Box::new(then_value),
                                            Box::new(else_value),
                                        ),
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        (1..=max_nodes)
            .find_map(|nodes| best[nodes][target_conditionals].clone().map(|expr| (nodes, expr)))
    }

    #[test]
    fn four_byte_conditional_and_depth_extremes() {
        let (nodes, triple_cond) = earliest_expr_with_conditionals(12, 3).unwrap();
        assert_eq!(nodes, 10);
        assert_eq!(conditional_count(&triple_cond), 3);
        assert_eq!(
            triple_cond.human_projection(),
            "IF-ATOM(0,0,IF-ATOM(0,0,IF-ATOM(0,0,0)))"
        );
        assert_eq!(rank_global(&triple_cond).unwrap(), 249_980_976);
        assert_eq!(encode_expr(&triple_cond).unwrap(), [0x0d, 0xe5, 0x67, 0x30]);
        assert!(earliest_expr_with_conditionals(12, 4).is_none());

        let mut deepest = Frame3MicroExpr::Zero;
        for _ in 0..11 {
            deepest = Frame3MicroExpr::Atom(Box::new(deepest));
        }
        assert_eq!(deepest.node_count(), 12);
        assert_eq!(expression_depth(&deepest), 12);
        assert_eq!(rank_global(&deepest).unwrap(), 3_149_929_038);
        assert_eq!(encode_expr(&deepest).unwrap(), [0xba, 0xbf, 0x19, 0x4e]);

        for (payload, expr, expected_source) in [
            ([0x0d, 0xe5, 0x67, 0x30], triple_cond, "0"),
            ([0xba, 0xbf, 0x19, 0x4e], deepest, "1"),
        ] {
            let source = expr.exact_words().join(" ");
            let actual = eval_parsed_expressions(
                &parse_canonical_binary(&source).unwrap(),
                &mut Session::default(),
            )
            .unwrap();
            let expected = eval_parsed_expressions(
                &parse_canonical_binary(expected_source).unwrap(),
                &mut Session::default(),
            )
            .unwrap();
            assert_eq!(actual.value, expected.value);
            assert_eq!(actual.output, expected.output);
            assert_eq!(decode_payload(&payload).unwrap(), expr);
        }
    }

    #[test]
    fn strongest_last_64k_f3m4_rank_is_executable() {
        let mut best: Option<((u32, usize, usize, usize, usize, usize, u32), Frame3MicroExpr)> =
            None;
        for local in 0xffff_0000u32..=0xffff_ffff {
            let payload = local.to_be_bytes();
            let expr = decode_payload(&payload).unwrap();
            let score = (
                expr.capability_mask().count_ones(),
                conditional_count(&expr),
                encoded_operation_count(&expr),
                expression_depth(&expr),
                expr.exact_words().len(),
                expr.node_count(),
                local,
            );
            if best.as_ref().is_none_or(|(current, _)| score > *current) {
                best = Some((score, expr));
            }
        }

        let (score, expr) = best.unwrap();
        assert_eq!(score.6, 0xffff_4f4f);
        assert_eq!(score.0, 5);
        assert_eq!(score.1, 1);
        assert_eq!(score.2, 14);
        assert_eq!(score.3, 6);
        assert_eq!(score.4, 52);
        assert_eq!(score.5, 12);
        assert_eq!(
            expr.human_projection(),
            "ATOM(CAR(CONS(CAR(CONS(0,ATOM(ATOM(QUOTE(NIL))))),IF-ATOM(ATOM(QUOTE(NIL)),QUOTE(NIL),QUOTE(NIL)))))"
        );
        eprintln!(
            "F3M4_FINAL64K_BEST={:08X}\tcap={}\tcond={}\tops={}\tdepth={}\twords={}\tnodes={}\t{}",
            score.6,
            score.0,
            score.1,
            score.2,
            score.3,
            score.4,
            score.5,
            expr.human_projection()
        );
        let payload = score.6.to_be_bytes();
        assert_eq!(encode_expr(&expr).unwrap(), payload);
        let source = expr.exact_words().join(" ");
        eval_parsed_expressions(
            &parse_canonical_binary(&source).unwrap(),
            &mut Session::default(),
        )
        .expect("F3M-4 final-page winner executes");
    }

    #[test]
    fn memorable_four_byte_payloads_are_real_programs() {
        for value in [
            0xcafe_babeu32,
            0xdead_beef,
            0xfeed_face,
            0xbaad_f00d,
            0xc0de_c0de,
            0x1234_5678,
            0xffff_ffff,
        ] {
            let payload = value.to_be_bytes();
            let expr = decode_payload(&payload).unwrap();
            let source = expr.exact_words().join(" ");
            let result = eval_parsed_expressions(
                &parse_canonical_binary(&source).unwrap(),
                &mut Session::default(),
            )
            .expect("memorable F3M-4 payload executes");
            eprintln!(
                "F3M4_MEM={value:08X}\tcap={}\tcond={}\tops={}\tdepth={}\twords={}\tnodes={}\tresult={:?}\t{}",
                expr.capability_mask().count_ones(),
                conditional_count(&expr),
                encoded_operation_count(&expr),
                expression_depth(&expr),
                expr.exact_words().len(),
                expr.node_count(),
                result.value,
                expr.human_projection()
            );
            assert_eq!(encode_expr(&expr).unwrap(), payload);
        }
    }

    #[test]
    fn four_byte_metric_saturating_full_basis_witness_executes() {
        // With CAP_ALL, CAR and CDR require two binary micro-nodes.
        // Three COND nodes plus those selectors need at least
        // 1 + 3*3 + 2*2 = 14 nodes, but F3M-4 ends inside node 12.
        // Therefore CAP_ALL can carry at most two CONDs in F3M-4.
        //
        // For two CONDs + CAR + CDR the minimum tree is 11 nodes:
        // 2 ternary + 2 binary internal nodes and 7 leaves. At 12 nodes
        // there is room for exactly one unary ATOM. Making all seven leaves
        // QUOTE forms yields the maximum encoded-operation count:
        // 2*2 (COND) + 2*2 (CAR/CDR+CONS) + 1 (ATOM) + 7 (QUOTE) = 16.
        let q = Frame3MicroExpr::QuoteZero;
        let inner_if = Frame3MicroExpr::IfAtom(
            Box::new(q.clone()),
            Box::new(q.clone()),
            Box::new(q.clone()),
        );
        let outer_if = Frame3MicroExpr::IfAtom(
            Box::new(q.clone()),
            Box::new(q.clone()),
            Box::new(inner_if),
        );
        let cdr = Frame3MicroExpr::CdrCons(Box::new(q.clone()), Box::new(outer_if));
        let car = Frame3MicroExpr::CarCons(Box::new(q), Box::new(cdr));
        let expr = Frame3MicroExpr::Atom(Box::new(car));

        assert_eq!(expr.capability_mask(), CAP_ALL);
        assert_eq!(conditional_count(&expr), 2);
        assert_eq!(encoded_operation_count(&expr), 16);
        assert_eq!(expression_depth(&expr), 6);
        assert_eq!(expr.exact_words().len(), 65);
        assert_eq!(expr.node_count(), 12);
        assert_eq!(
            expr.human_projection(),
            "ATOM(CAR(CONS(QUOTE(0),CDR(CONS(QUOTE(0),IF-ATOM(QUOTE(0),QUOTE(0),IF-ATOM(QUOTE(0),QUOTE(0),QUOTE(0))))))))"
        );
        assert_eq!(encode_expr(&expr).unwrap(), [0xf5, 0xa7, 0xbc, 0x39]);

        let source = expr.exact_words().join(" ");
        let actual = eval_parsed_expressions(
            &parse_canonical_binary(&source).unwrap(),
            &mut Session::default(),
        )
        .unwrap();
        let expected = eval_parsed_expressions(
            &parse_canonical_binary("1").unwrap(),
            &mut Session::default(),
        )
        .unwrap();
        assert_eq!(actual.value, expected.value);
        assert_eq!(actual.output, expected.output);
    }

    #[test]
    fn strongest_last_64k_f3m3_rank_is_full_basis_and_executes() {
        // Transparent lexicographic search score:
        // capability diversity, COND count, encoded operation count, depth,
        // exact-word count, node count, then the later physical rank.
        let mut best: Option<((u32, usize, usize, usize, usize, usize, u32), Frame3MicroExpr)> =
            None;
        for local in 0xff_00_00u32..=0xff_ff_ff {
            let payload = [
                ((local >> 16) & 0xff) as u8,
                ((local >> 8) & 0xff) as u8,
                (local & 0xff) as u8,
            ];
            let expr = decode_payload(&payload).unwrap();
            let score = (
                expr.capability_mask().count_ones(),
                conditional_count(&expr),
                encoded_operation_count(&expr),
                expression_depth(&expr),
                expr.exact_words().len(),
                expr.node_count(),
                local,
            );
            if best.as_ref().is_none_or(|(current, _)| score > *current) {
                best = Some((score, expr));
            }
        }

        let (score, expr) = best.unwrap();
        assert_eq!(score.6, 0xff_fb_37);
        assert_eq!(expr.capability_mask(), CAP_ALL);
        assert_eq!(score.0, 6);
        assert_eq!(score.1, 1);
        assert_eq!(score.2, 11);
        assert_eq!(score.3, 4);
        assert_eq!(score.4, 43);
        assert_eq!(score.5, 9);
        assert_eq!(
            expr.human_projection(),
            "CAR(CONS(CDR(CONS(1,QUOTE(1))),ATOM(IF-ATOM(QUOTE(NIL),QUOTE(NIL),QUOTE(NIL)))))"
        );

        let payload = [0xff, 0xfb, 0x37];
        assert_eq!(encode_expr(&expr).unwrap(), payload);
        let source = expr.exact_words().join(" ");
        let parsed = parse_canonical_binary(&source).expect("boundary F3M-3 source");
        let actual = eval_parsed_expressions(&parsed, &mut Session::default())
            .expect("boundary F3M-3 witness executes");
        let expected = eval_parsed_expressions(
            &parse_canonical_binary("1").unwrap(),
            &mut Session::default(),
        )
        .unwrap();
        assert_eq!(actual.value, expected.value);
        assert_eq!(actual.output, expected.output);

        // The literal endpoint is later but less expressive by this score.
        let endpoint = decode_payload(&[0xff, 0xff, 0xff]).unwrap();
        assert_eq!(endpoint.capability_mask().count_ones(), 4);
        assert_eq!(conditional_count(&endpoint), 0);
    }

    #[test]
    fn first_full_capability_basis_fits_in_three_bytes_and_executes() {
        let first = first_global_rank_with_capabilities(CAP_ALL).unwrap();
        assert_eq!(first, 1_523_043);

        let local = first - F3M3_OFFSET;
        assert_eq!(local, 0x16_3c_63);
        let payload = [0x16, 0x3c, 0x63];

        let expr = decode_payload(&payload).unwrap();
        assert_eq!(expr.capability_mask(), CAP_ALL);
        assert_eq!(rank_global(&expr).unwrap(), first);
        assert_eq!(encode_expr(&expr).unwrap(), payload);

        let source = expr.exact_words().join(" ");
        let parsed = parse_canonical_binary(&source).expect("full-basis F3M-3 source");
        let actual = eval_parsed_expressions(&parsed, &mut Session::default())
            .expect("full-basis F3M-3 program executes");
        assert_eq!(
            expr.human_projection(),
            "CAR(CONS(0,CDR(CONS(0,IF-ATOM(0,0,QUOTE(0))))))"
        );
        let expected = eval_parsed_expressions(
            &parse_canonical_binary("0").unwrap(),
            &mut Session::default(),
        )
        .unwrap();
        assert_eq!(actual.value, expected.value);
        assert_eq!(actual.output, expected.output);
    }

    #[test]
    fn three_byte_conditional_is_real_sens_computation() {
        let payload = [0x2f, 0x18, 0x59];
        let expr = decode_payload(&payload).unwrap();
        assert_eq!(
            expr.human_projection(),
            "IF-ATOM(QUOTE(0),CAR(CONS(QUOTE(1),1)),CDR(CONS(QUOTE(NIL),QUOTE(0))))"
        );
        let source = expr.exact_words().join(" ");
        let parsed = parse_canonical_binary(&source).expect("F3M exact SENS source");
        let actual = eval_parsed_expressions(&parsed, &mut Session::default())
            .expect("F3M decoded program must execute");
        let expected = eval_parsed_expressions(
            &parse_canonical_binary("1").unwrap(),
            &mut Session::default(),
        )
        .unwrap();
        assert_eq!(actual.value, expected.value);
        assert_eq!(actual.output, expected.output);
        assert_eq!(encode_expr(&expr).unwrap(), payload);
    }
}
