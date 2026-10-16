//! Retired standalone nine-bit carrier.
//! The width now lives in the shared carrier: `Bit9 = Bits<9>` (crates/sens/src/bits.rs).
//! Kept as a module so existing `crate::Bit9` paths still resolve via the re-export.

pub use crate::bits::Bit9;
