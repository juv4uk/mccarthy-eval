//! The McCarthy primitives (`eq`, `car`, `cdr`, `cons`, `cond`, `quote`'s helper),
//! plus the small compatibility/bootstrap surface and host mechanisms that still
//! require Rust. Ordinary primitive dispatch is keyed by exact SENS identity;
//! this module does not install name-bound root builtins.

mod codepoint;
mod core;
pub(crate) mod digest;
mod io;
pub mod json;
mod strings;

pub use core::exact_arity;
pub(super) use codepoint::{codepoint_to_string_values, string_to_codepoint_values};
pub(super) use core::{
    atom_value, car_value, cdr_value, cons_values, eq_values, evaluate_definition,
    evaluate_domain_cond, quoted,
};
pub(super) use digest::sha256_hex_values;
pub(super) use io::{
    eval_values, princ_values, print_values, read_all_values, read_values,
    write_to_string_values,
};
pub(super) use strings::{
    evaluate_string_slice, string_append_values, string_first_values,
    string_predicate_values, string_rest_values, string_to_symbol_values, symbol_to_string_values,
};
