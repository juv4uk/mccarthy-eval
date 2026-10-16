// GENERATED from contracts/numeric-domains-ratification.lisp — DO NOT EDIT BY HAND.
// Authority: contracts/numeric-domains-ratification.lisp (D(24 * 2^n), n >= 0).
// Rust holds bytes; it does not decide codes.

/// Canonical wide numeric-domain widths, in contract order.
pub const NUMERIC_DOMAIN_WIDTHS: [u16; 4] = [ 24, 48, 96, 192 ];

/// The first (narrowest) ratified numeric domain width.
pub const FIRST_NUMERIC_DOMAIN_WIDTH: u16 = 24;

/// Next rung from any ratified width (contract: next-width = double).
pub const fn next_width(w: u16) -> u16 { w * 2 }
