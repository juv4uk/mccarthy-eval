//! Prefix-stable five-byte F3M extension.
//!
//! F3M-1..4 keep their original dense grammar and byte->program identity.
//! F3M-5 is a separate extension rank space containing only programs that use
//! at least one newly admitted constructor:
//!   - EQ-ATOM: D3 EQ applied to two exact-D1 ATOM results, therefore total;
//!   - LAMBDA0: D4 LAMBDA with an empty parameter list and a closed body.
//!
//! The remaining constructors are the frozen F3M-1..4 D1/D2/D3 basis. This
//! means the five-byte grammar exercises every D3 primitive while D2 remains
//! the structural list/framing layer rather than a callable operation.
//!
//! Extension ordering is canonical by micro-node count, constructor family,
//! child node split, extension-state tuple, then child rank. The extension
//! state is explicit so no F3M-1..4 rank can move when this grammar grows.

use crate::frame3_micro_enumerator::{Frame3MicroEnumerateError, Frame3MicroExpr};

pub const F3M5_CAPACITY: u128 = 1u128 << 40;
const COUNT_LIMIT: usize = 13;

type Counts = [[u128; 2]; COUNT_LIMIT + 1];

fn checked_add(a: u128, b: u128) -> Result<u128, Frame3MicroEnumerateError> {
    a.checked_add(b)
        .ok_or(Frame3MicroEnumerateError::InternalCountOverflow)
}

fn checked_mul(a: u128, b: u128) -> Result<u128, Frame3MicroEnumerateError> {
    a.checked_mul(b)
        .ok_or(Frame3MicroEnumerateError::InternalCountOverflow)
}

fn total_at(counts: &Counts, nodes: usize) -> Result<u128, Frame3MicroEnumerateError> {
    checked_add(counts[nodes][0], counts[nodes][1])
}

fn count_table() -> Result<Counts, Frame3MicroEnumerateError> {
    let mut counts = [[0u128; 2]; COUNT_LIMIT + 1];
    counts[1][0] = 6;

    for nodes in 2..=COUNT_LIMIT {
        // Legacy state: exactly the frozen F3M-1..4 grammar.
        let mut legacy = counts[nodes - 1][0]; // ATOM
        for left_nodes in 1..nodes - 1 {
            let right_nodes = nodes - 1 - left_nodes;
            let pair = checked_mul(counts[left_nodes][0], counts[right_nodes][0])?;
            legacy = checked_add(legacy, checked_mul(3, pair)?)?;
        }
        for test_nodes in 1..nodes.saturating_sub(2) {
            for then_nodes in 1..nodes - 1 - test_nodes {
                let else_nodes = nodes - 1 - test_nodes - then_nodes;
                if else_nodes == 0 {
                    continue;
                }
                let block = checked_mul(
                    checked_mul(counts[test_nodes][0], counts[then_nodes][0])?,
                    counts[else_nodes][0],
                )?;
                legacy = checked_add(legacy, block)?;
            }
        }

        // Extended state.
        // ATOM(new-child) + LAMBDA0(any-child).
        let mut extended = checked_add(counts[nodes - 1][1], total_at(&counts, nodes - 1)?)?;

        for left_nodes in 1..nodes - 1 {
            let right_nodes = nodes - 1 - left_nodes;
            let left_total = total_at(&counts, left_nodes)?;
            let right_total = total_at(&counts, right_nodes)?;
            let full_pair = checked_mul(left_total, right_total)?;
            let legacy_pair = checked_mul(counts[left_nodes][0], counts[right_nodes][0])?;
            let extended_pair = full_pair
                .checked_sub(legacy_pair)
                .ok_or(Frame3MicroEnumerateError::InternalCountOverflow)?;

            // CONS / CAR(CONS) / CDR(CONS) with at least one extended child.
            extended = checked_add(extended, checked_mul(3, extended_pair)?)?;
            // EQ-ATOM is itself new, so every child-state pair is admitted.
            extended = checked_add(extended, full_pair)?;
        }

        for test_nodes in 1..nodes.saturating_sub(2) {
            for then_nodes in 1..nodes - 1 - test_nodes {
                let else_nodes = nodes - 1 - test_nodes - then_nodes;
                if else_nodes == 0 {
                    continue;
                }
                let full = checked_mul(
                    checked_mul(
                        total_at(&counts, test_nodes)?,
                        total_at(&counts, then_nodes)?,
                    )?,
                    total_at(&counts, else_nodes)?,
                )?;
                let old = checked_mul(
                    checked_mul(counts[test_nodes][0], counts[then_nodes][0])?,
                    counts[else_nodes][0],
                )?;
                extended = checked_add(
                    extended,
                    full.checked_sub(old)
                        .ok_or(Frame3MicroEnumerateError::InternalCountOverflow)?,
                )?;
            }
        }

        counts[nodes] = [legacy, extended];
    }

    Ok(counts)
}

pub(crate) fn contains_extension(expr: &Frame3MicroExpr) -> bool {
    match expr {
        Frame3MicroExpr::Lambda0(_) | Frame3MicroExpr::EqAtom(_, _) => true,
        Frame3MicroExpr::Atom(value) => contains_extension(value),
        Frame3MicroExpr::Cons(left, right)
        | Frame3MicroExpr::CarCons(left, right)
        | Frame3MicroExpr::CdrCons(left, right) => {
            contains_extension(left) || contains_extension(right)
        }
        Frame3MicroExpr::IfAtom(test, then_value, else_value) => {
            contains_extension(test)
                || contains_extension(then_value)
                || contains_extension(else_value)
        }
        _ => false,
    }
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

fn unrank_legacy(
    nodes: usize,
    mut rank: u128,
    counts: &Counts,
) -> Result<Frame3MicroExpr, Frame3MicroEnumerateError> {
    if nodes == 1 {
        return leaf_from_rank(rank);
    }
    if nodes > COUNT_LIMIT || rank >= counts[nodes][0] {
        return Err(Frame3MicroEnumerateError::RankOutOfRange);
    }

    let atom_block = counts[nodes - 1][0];
    if rank < atom_block {
        return Ok(Frame3MicroExpr::Atom(Box::new(unrank_legacy(
            nodes - 1,
            rank,
            counts,
        )?)));
    }
    rank -= atom_block;

    for family in 0..3 {
        for left_nodes in 1..nodes - 1 {
            let right_nodes = nodes - 1 - left_nodes;
            let block = checked_mul(counts[left_nodes][0], counts[right_nodes][0])?;
            if rank < block {
                let left_rank = rank / counts[right_nodes][0];
                let right_rank = rank % counts[right_nodes][0];
                let left = Box::new(unrank_legacy(left_nodes, left_rank, counts)?);
                let right = Box::new(unrank_legacy(right_nodes, right_rank, counts)?);
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
            let then_else = checked_mul(counts[then_nodes][0], counts[else_nodes][0])?;
            let block = checked_mul(counts[test_nodes][0], then_else)?;
            if rank < block {
                let test_rank = rank / then_else;
                let remainder = rank % then_else;
                let then_rank = remainder / counts[else_nodes][0];
                let else_rank = remainder % counts[else_nodes][0];
                return Ok(Frame3MicroExpr::IfAtom(
                    Box::new(unrank_legacy(test_nodes, test_rank, counts)?),
                    Box::new(unrank_legacy(then_nodes, then_rank, counts)?),
                    Box::new(unrank_legacy(else_nodes, else_rank, counts)?),
                ));
            }
            rank -= block;
        }
    }

    Err(Frame3MicroEnumerateError::RankOutOfRange)
}

fn rank_legacy(
    expr: &Frame3MicroExpr,
    counts: &Counts,
) -> Result<u128, Frame3MicroEnumerateError> {
    let nodes = expr.node_count();
    if nodes == 1 {
        return leaf_rank(expr).ok_or(Frame3MicroEnumerateError::RankOutOfRange);
    }
    if nodes > COUNT_LIMIT || contains_extension(expr) {
        return Err(Frame3MicroEnumerateError::RankOutOfRange);
    }

    if let Frame3MicroExpr::Atom(value) = expr {
        return rank_legacy(value, counts);
    }
    let mut rank = counts[nodes - 1][0];

    let mut binary_pairs = 0u128;
    for left_nodes in 1..nodes - 1 {
        let right_nodes = nodes - 1 - left_nodes;
        binary_pairs = checked_add(
            binary_pairs,
            checked_mul(counts[left_nodes][0], counts[right_nodes][0])?,
        )?;
    }

    let binary = match expr {
        Frame3MicroExpr::Cons(left, right) => Some((0u128, left, right)),
        Frame3MicroExpr::CarCons(left, right) => Some((1u128, left, right)),
        Frame3MicroExpr::CdrCons(left, right) => Some((2u128, left, right)),
        _ => None,
    };
    if let Some((family, left, right)) = binary {
        rank = checked_add(rank, checked_mul(family, binary_pairs)?)?;
        let left_nodes = left.node_count();
        let right_nodes = right.node_count();
        for earlier_left in 1..left_nodes {
            let earlier_right = nodes - 1 - earlier_left;
            rank = checked_add(
                rank,
                checked_mul(counts[earlier_left][0], counts[earlier_right][0])?,
            )?;
        }
        rank = checked_add(
            rank,
            checked_mul(rank_legacy(left, counts)?, counts[right_nodes][0])?,
        )?;
        return checked_add(rank, rank_legacy(right, counts)?);
    }

    rank = checked_add(rank, checked_mul(3, binary_pairs)?)?;

    if let Frame3MicroExpr::IfAtom(test, then_value, else_value) = expr {
        let test_nodes = test.node_count();
        let then_nodes = then_value.node_count();
        let else_nodes = else_value.node_count();

        for earlier_test in 1..test_nodes {
            for earlier_then in 1..nodes - 1 - earlier_test {
                let earlier_else = nodes - 1 - earlier_test - earlier_then;
                if earlier_else == 0 {
                    continue;
                }
                rank = checked_add(
                    rank,
                    checked_mul(
                        checked_mul(counts[earlier_test][0], counts[earlier_then][0])?,
                        counts[earlier_else][0],
                    )?,
                )?;
            }
        }
        for earlier_then in 1..then_nodes {
            let earlier_else = nodes - 1 - test_nodes - earlier_then;
            if earlier_else == 0 {
                continue;
            }
            rank = checked_add(
                rank,
                checked_mul(
                    checked_mul(counts[test_nodes][0], counts[earlier_then][0])?,
                    counts[earlier_else][0],
                )?,
            )?;
        }

        let then_else = checked_mul(counts[then_nodes][0], counts[else_nodes][0])?;
        rank = checked_add(rank, checked_mul(rank_legacy(test, counts)?, then_else)?)?;
        rank = checked_add(
            rank,
            checked_mul(rank_legacy(then_value, counts)?, counts[else_nodes][0])?,
        )?;
        return checked_add(rank, rank_legacy(else_value, counts)?);
    }

    Err(Frame3MicroEnumerateError::RankOutOfRange)
}

fn unrank_state(
    nodes: usize,
    extended: bool,
    mut rank: u128,
    counts: &Counts,
) -> Result<Frame3MicroExpr, Frame3MicroEnumerateError> {
    if !extended {
        return unrank_legacy(nodes, rank, counts);
    }
    if nodes <= 1 || nodes > COUNT_LIMIT || rank >= counts[nodes][1] {
        return Err(Frame3MicroEnumerateError::RankOutOfRange);
    }

    // ATOM with an extended child.
    let block = counts[nodes - 1][1];
    if rank < block {
        return Ok(Frame3MicroExpr::Atom(Box::new(unrank_state(
            nodes - 1,
            true,
            rank,
            counts,
        )?)));
    }
    rank -= block;

    // LAMBDA0 with legacy child first, then extended child.
    for child_extended in [false, true] {
        let block = counts[nodes - 1][usize::from(child_extended)];
        if rank < block {
            return Ok(Frame3MicroExpr::Lambda0(Box::new(unrank_state(
                nodes - 1,
                child_extended,
                rank,
                counts,
            )?)));
        }
        rank -= block;
    }

    // Old binary constructors require at least one extended child.
    let extended_pairs = [(false, true), (true, false), (true, true)];
    for family in 0..3 {
        for left_nodes in 1..nodes - 1 {
            let right_nodes = nodes - 1 - left_nodes;
            for (left_extended, right_extended) in extended_pairs {
                let right_count = counts[right_nodes][usize::from(right_extended)];
                let block = checked_mul(
                    counts[left_nodes][usize::from(left_extended)],
                    right_count,
                )?;
                if rank < block {
                    let left_rank = rank / right_count;
                    let right_rank = rank % right_count;
                    let left = Box::new(unrank_state(
                        left_nodes,
                        left_extended,
                        left_rank,
                        counts,
                    )?);
                    let right = Box::new(unrank_state(
                        right_nodes,
                        right_extended,
                        right_rank,
                        counts,
                    )?);
                    return Ok(match family {
                        0 => Frame3MicroExpr::Cons(left, right),
                        1 => Frame3MicroExpr::CarCons(left, right),
                        _ => Frame3MicroExpr::CdrCons(left, right),
                    });
                }
                rank -= block;
            }
        }
    }

    // EQ-ATOM is itself extended, so all child-state pairs are admitted.
    let all_pairs = [(false, false), (false, true), (true, false), (true, true)];
    for left_nodes in 1..nodes - 1 {
        let right_nodes = nodes - 1 - left_nodes;
        for (left_extended, right_extended) in all_pairs {
            let right_count = counts[right_nodes][usize::from(right_extended)];
            let block = checked_mul(
                counts[left_nodes][usize::from(left_extended)],
                right_count,
            )?;
            if rank < block {
                let left_rank = rank / right_count;
                let right_rank = rank % right_count;
                return Ok(Frame3MicroExpr::EqAtom(
                    Box::new(unrank_state(
                        left_nodes,
                        left_extended,
                        left_rank,
                        counts,
                    )?),
                    Box::new(unrank_state(
                        right_nodes,
                        right_extended,
                        right_rank,
                        counts,
                    )?),
                ));
            }
            rank -= block;
        }
    }

    // IF-ATOM is an old constructor: at least one child must be extended.
    for test_nodes in 1..nodes.saturating_sub(2) {
        for then_nodes in 1..nodes - 1 - test_nodes {
            let else_nodes = nodes - 1 - test_nodes - then_nodes;
            if else_nodes == 0 {
                continue;
            }
            for mask in 1u8..=7 {
                let test_extended = mask & 0b100 != 0;
                let then_extended = mask & 0b010 != 0;
                let else_extended = mask & 0b001 != 0;
                let then_count = counts[then_nodes][usize::from(then_extended)];
                let else_count = counts[else_nodes][usize::from(else_extended)];
                let then_else = checked_mul(then_count, else_count)?;
                let block = checked_mul(
                    counts[test_nodes][usize::from(test_extended)],
                    then_else,
                )?;
                if rank < block {
                    let test_rank = rank / then_else;
                    let remainder = rank % then_else;
                    let then_rank = remainder / else_count;
                    let else_rank = remainder % else_count;
                    return Ok(Frame3MicroExpr::IfAtom(
                        Box::new(unrank_state(
                            test_nodes,
                            test_extended,
                            test_rank,
                            counts,
                        )?),
                        Box::new(unrank_state(
                            then_nodes,
                            then_extended,
                            then_rank,
                            counts,
                        )?),
                        Box::new(unrank_state(
                            else_nodes,
                            else_extended,
                            else_rank,
                            counts,
                        )?),
                    ));
                }
                rank -= block;
            }
        }
    }

    Err(Frame3MicroEnumerateError::RankOutOfRange)
}

fn rank_extended_exact(
    expr: &Frame3MicroExpr,
    counts: &Counts,
) -> Result<u128, Frame3MicroEnumerateError> {
    if !contains_extension(expr) {
        return Err(Frame3MicroEnumerateError::RankOutOfRange);
    }
    let nodes = expr.node_count();
    if nodes <= 1 || nodes > COUNT_LIMIT {
        return Err(Frame3MicroEnumerateError::RankOutOfRange);
    }

    // ATOM(new child).
    if let Frame3MicroExpr::Atom(value) = expr {
        if contains_extension(value) {
            return rank_extended_exact(value, counts);
        }
    }
    let mut rank = counts[nodes - 1][1];

    // LAMBDA0(legacy child), LAMBDA0(extended child).
    if let Frame3MicroExpr::Lambda0(value) = expr {
        let child_extended = contains_extension(value);
        if child_extended {
            rank = checked_add(rank, counts[nodes - 1][0])?;
            return checked_add(rank, rank_extended_exact(value, counts)?);
        }
        return checked_add(rank, rank_legacy(value, counts)?);
    }
    rank = checked_add(rank, total_at(counts, nodes - 1)?)?;

    let extended_pairs = [(false, true), (true, false), (true, true)];
    let binary = match expr {
        Frame3MicroExpr::Cons(left, right) => Some((0usize, left, right)),
        Frame3MicroExpr::CarCons(left, right) => Some((1usize, left, right)),
        Frame3MicroExpr::CdrCons(left, right) => Some((2usize, left, right)),
        _ => None,
    };

    for family in 0..3 {
        for left_nodes in 1..nodes - 1 {
            let right_nodes = nodes - 1 - left_nodes;
            for (left_extended, right_extended) in extended_pairs {
                let block = checked_mul(
                    counts[left_nodes][usize::from(left_extended)],
                    counts[right_nodes][usize::from(right_extended)],
                )?;
                if let Some((actual_family, left, right)) = binary {
                    let actual_states = (contains_extension(left), contains_extension(right));
                    if family == actual_family
                        && left_nodes == left.node_count()
                        && (left_extended, right_extended) == actual_states
                    {
                        let left_rank = if left_extended {
                            rank_extended_exact(left, counts)?
                        } else {
                            rank_legacy(left, counts)?
                        };
                        let right_rank = if right_extended {
                            rank_extended_exact(right, counts)?
                        } else {
                            rank_legacy(right, counts)?
                        };
                        rank = checked_add(
                            rank,
                            checked_mul(
                                left_rank,
                                counts[right_nodes][usize::from(right_extended)],
                            )?,
                        )?;
                        return checked_add(rank, right_rank);
                    }
                }
                rank = checked_add(rank, block)?;
            }
        }
    }

    // EQ-ATOM family.
    let all_pairs = [(false, false), (false, true), (true, false), (true, true)];
    for left_nodes in 1..nodes - 1 {
        let right_nodes = nodes - 1 - left_nodes;
        for (left_extended, right_extended) in all_pairs {
            let block = checked_mul(
                counts[left_nodes][usize::from(left_extended)],
                counts[right_nodes][usize::from(right_extended)],
            )?;
            if let Frame3MicroExpr::EqAtom(left, right) = expr {
                let actual_states = (contains_extension(left), contains_extension(right));
                if left_nodes == left.node_count()
                    && (left_extended, right_extended) == actual_states
                {
                    let left_rank = if left_extended {
                        rank_extended_exact(left, counts)?
                    } else {
                        rank_legacy(left, counts)?
                    };
                    let right_rank = if right_extended {
                        rank_extended_exact(right, counts)?
                    } else {
                        rank_legacy(right, counts)?
                    };
                    rank = checked_add(
                        rank,
                        checked_mul(
                            left_rank,
                            counts[right_nodes][usize::from(right_extended)],
                        )?,
                    )?;
                    return checked_add(rank, right_rank);
                }
            }
            rank = checked_add(rank, block)?;
        }
    }

    // IF-ATOM with at least one extended child.
    if let Frame3MicroExpr::IfAtom(test, then_value, else_value) = expr {
        let actual_nodes = (
            test.node_count(),
            then_value.node_count(),
            else_value.node_count(),
        );
        let actual_mask = (u8::from(contains_extension(test)) << 2)
            | (u8::from(contains_extension(then_value)) << 1)
            | u8::from(contains_extension(else_value));
        if actual_mask == 0 {
            return Err(Frame3MicroEnumerateError::RankOutOfRange);
        }

        for test_nodes in 1..nodes.saturating_sub(2) {
            for then_nodes in 1..nodes - 1 - test_nodes {
                let else_nodes = nodes - 1 - test_nodes - then_nodes;
                if else_nodes == 0 {
                    continue;
                }
                for mask in 1u8..=7 {
                    let test_extended = mask & 0b100 != 0;
                    let then_extended = mask & 0b010 != 0;
                    let else_extended = mask & 0b001 != 0;
                    let block = checked_mul(
                        checked_mul(
                            counts[test_nodes][usize::from(test_extended)],
                            counts[then_nodes][usize::from(then_extended)],
                        )?,
                        counts[else_nodes][usize::from(else_extended)],
                    )?;
                    if (test_nodes, then_nodes, else_nodes) == actual_nodes
                        && mask == actual_mask
                    {
                        let test_rank = if test_extended {
                            rank_extended_exact(test, counts)?
                        } else {
                            rank_legacy(test, counts)?
                        };
                        let then_rank = if then_extended {
                            rank_extended_exact(then_value, counts)?
                        } else {
                            rank_legacy(then_value, counts)?
                        };
                        let else_rank = if else_extended {
                            rank_extended_exact(else_value, counts)?
                        } else {
                            rank_legacy(else_value, counts)?
                        };
                        let then_else = checked_mul(
                            counts[then_nodes][usize::from(then_extended)],
                            counts[else_nodes][usize::from(else_extended)],
                        )?;
                        rank = checked_add(rank, checked_mul(test_rank, then_else)?)?;
                        rank = checked_add(
                            rank,
                            checked_mul(
                                then_rank,
                                counts[else_nodes][usize::from(else_extended)],
                            )?,
                        )?;
                        return checked_add(rank, else_rank);
                    }
                    rank = checked_add(rank, block)?;
                }
            }
        }
    }

    Err(Frame3MicroEnumerateError::RankOutOfRange)
}

pub(crate) fn decode_extension(
    local_rank: u128,
) -> Result<Frame3MicroExpr, Frame3MicroEnumerateError> {
    if local_rank >= F3M5_CAPACITY {
        return Err(Frame3MicroEnumerateError::RankOutOfRange);
    }
    let counts = count_table()?;
    let mut remaining = local_rank;
    for nodes in 1..=COUNT_LIMIT {
        let block = counts[nodes][1];
        if remaining < block {
            return unrank_state(nodes, true, remaining, &counts);
        }
        remaining -= block;
    }
    Err(Frame3MicroEnumerateError::RankOutOfRange)
}

pub(crate) fn encode_extension(
    expr: &Frame3MicroExpr,
) -> Result<u128, Frame3MicroEnumerateError> {
    if !contains_extension(expr) {
        return Err(Frame3MicroEnumerateError::RankOutOfRange);
    }
    let counts = count_table()?;
    let nodes = expr.node_count();
    if nodes > COUNT_LIMIT {
        return Err(Frame3MicroEnumerateError::RankOutOfRange);
    }
    let mut rank = 0u128;
    for n in 1..nodes {
        rank = checked_add(rank, counts[n][1])?;
    }
    rank = checked_add(rank, rank_extended_exact(expr, &counts)?)?;
    if rank >= F3M5_CAPACITY {
        return Err(Frame3MicroEnumerateError::RankOutOfRange);
    }
    Ok(rank)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn five_byte_extension_capacity_crosses_full_40_bit_window_by_node_13() {
        let counts = count_table().unwrap();
        let through_12 = (1..=12).map(|nodes| counts[nodes][1]).sum::<u128>();
        let through_13 = through_12 + counts[13][1];
        assert_eq!(counts[2][1], 6);
        assert_eq!(through_12, 197_104_694_850);
        assert_eq!(counts[13][1], 2_135_013_404_850);
        assert!(through_12 < F3M5_CAPACITY);
        assert!(through_13 >= F3M5_CAPACITY);
    }

    #[test]
    fn de_bruijn_lambda1_probe_fits_a_full_five_byte_window_by_node_13() {
        // Research-only capacity probe for a *real* one-argument lambda.
        // A bound variable is represented canonically by its de Bruijn index,
        // so alpha-renaming cannot create duplicate programs. This does NOT
        // alter the admitted F3M-5 mapping or SENS LAMBDA semantics.
        const NODES: usize = 13;
        const DEPTHS: usize = 15;
        let mut closed = [[0u128; DEPTHS]; NODES + 1];

        // Six frozen F3M leaves plus one canonical bound-variable leaf for
        // every in-scope de Bruijn index 0..depth-1.
        for depth in 0..DEPTHS {
            closed[1][depth] = 6 + depth as u128;
        }

        for nodes in 2..=NODES {
            for depth in (0..DEPTHS - 1).rev() {
                // ATOM(body) + LAMBDA1(body-at-depth+1).
                let mut total = closed[nodes - 1][depth]
                    .checked_add(closed[nodes - 1][depth + 1])
                    .expect("probe count");

                // CONS, CAR(CONS), CDR(CONS), EQ(ATOM,ATOM).
                let mut binary_pairs = 0u128;
                for left_nodes in 1..nodes - 1 {
                    let right_nodes = nodes - 1 - left_nodes;
                    binary_pairs = binary_pairs
                        .checked_add(
                            closed[left_nodes][depth]
                                .checked_mul(closed[right_nodes][depth])
                                .expect("probe pair count"),
                        )
                        .expect("probe pair sum");
                }
                total = total
                    .checked_add(binary_pairs.checked_mul(4).expect("four binary families"))
                    .expect("probe binary sum");

                // Total IF-ATOM, same ternary split law as current F3M.
                for test_nodes in 1..nodes.saturating_sub(2) {
                    for then_nodes in 1..nodes - 1 - test_nodes {
                        let else_nodes = nodes - 1 - test_nodes - then_nodes;
                        if else_nodes == 0 {
                            continue;
                        }
                        total = total
                            .checked_add(
                                closed[test_nodes][depth]
                                    .checked_mul(closed[then_nodes][depth])
                                    .and_then(|v| v.checked_mul(closed[else_nodes][depth]))
                                    .expect("probe conditional count"),
                            )
                            .expect("probe conditional sum");
                    }
                }
                closed[nodes][depth] = total;
            }
        }

        let legacy = count_table().expect("current F3M counts");
        let through_12_total = (1..=12).map(|n| closed[n][0]).sum::<u128>();
        let through_13_total = through_12_total + closed[13][0];
        let through_12_legacy = (1..=12).map(|n| legacy[n][0]).sum::<u128>();
        let through_13_legacy = through_12_legacy + legacy[13][0];

        let through_12_extension = through_12_total - through_12_legacy;
        let through_13_extension = through_13_total - through_13_legacy;

        assert_eq!(through_12_extension, 298_460_447_325);
        assert_eq!(through_13_extension, 3_706_360_960_907);
        assert!(through_12_extension < F3M5_CAPACITY);
        assert!(through_13_extension >= F3M5_CAPACITY);
    }

    #[test]
    fn first_extension_programs_are_lambda_and_eq_capable() {
        let zero = decode_extension(0).unwrap();
        assert_eq!(
            zero,
            Frame3MicroExpr::Lambda0(Box::new(Frame3MicroExpr::Zero))
        );
        // Pin the first five-byte mapping so future grammar edits cannot
        // silently remap the profile.
        assert_eq!(encode_extension(&zero).unwrap(), 0);

        let lambda = Frame3MicroExpr::Lambda0(Box::new(Frame3MicroExpr::QuoteNil));
        let lambda_rank = encode_extension(&lambda).unwrap();
        assert_eq!(decode_extension(lambda_rank).unwrap(), lambda);

        let eq = Frame3MicroExpr::EqAtom(
            Box::new(Frame3MicroExpr::Zero),
            Box::new(Frame3MicroExpr::One),
        );
        let eq_rank = encode_extension(&eq).unwrap();
        assert_eq!(decode_extension(eq_rank).unwrap(), eq);
    }
}
