//! Legacy exact-eight SENS carrier -- kept as the MACRO BRIDGE only.
//!
//! The macro `sens!` belongs to the LANGUAGE (SENS). In Rust it is the single
//! bridge from eight source 0/1 characters to a carrier value; it performs no
//! domain inference. Canonical semantic identity is the exact-domain carrier
//! `CoreDomainIdentity` (D1-D9, `crates/sens/src/domain_identity.rs`).
//!
//! `Sens8`/`Sens` remain so still-migrating call-sites `crate::sens!(bits)`
//! keep compiling; the carrier is carried to the domain layer explicitly.

pub use crate::domain_identity::CoreDomainIdentity;

use std::fmt;

/// Legacy exact-eight carrier used by the `sens!` bridge.
#[repr(transparent)]
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct Sens8(pub(crate) u8);

/// Historical compatibility alias.
pub type Sens = Sens8;

impl Sens8 {
    #[doc(hidden)]
    pub const fn __from_macro_bits(bits: &str) -> Self {
        let bytes = bits.as_bytes();
        assert!(bytes.len() == 8, "sens! requires exactly eight bare binary digits");
        let mut packed = 0u8;
        let mut index = 0usize;
        while index < 8 {
            let byte = bytes[index];
            assert!(byte == b'0' || byte == b'1', "sens! accepts only eight bare 0/1 digits");
            packed = (packed << 1) | (byte - b'0');
            index += 1;
        }
        Self(packed)
    }
    pub(crate) const fn from_exact_bits(bits: &str) -> Option<Self> {
        let bytes = bits.as_bytes();
        if bytes.len() != 8 { return None; }
        let mut packed = 0u8; let mut index = 0usize;
        while index < 8 {
            let byte = bytes[index];
            if byte != b'0' && byte != b'1' { return None; }
            packed = (packed << 1) | (byte - b'0'); index += 1;
        }
        Some(Self(packed))
    }
    pub(crate) const fn from_packed_byte(byte: u8) -> Self { Self(byte) }
    pub const fn packed_byte(self) -> u8 { self.0 }
}

impl fmt::Display for Sens8 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{:08b}", self.0) }
}
impl fmt::Debug for Sens8 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "Sens8({self})") }
}

/// Bridge from eight source 0/1 characters (the language macro lives in SENS).
#[macro_export]
macro_rules! sens {
    ($bits:literal) => {{
        const SENS: $crate::Sens8 = $crate::Sens8::__from_macro_bits(stringify!($bits));
        SENS
    }};
}
