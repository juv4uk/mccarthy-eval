//! Frame3-Micro (F3M) physical-profile selector.
//!
//! The profile is selected ONLY by the exact physical payload length.
//! Payload bytes are never sniffed for magic/version markers:
//!   1 byte -> F3M-1
//!   2 bytes -> F3M-2
//!   3 bytes -> F3M-3
//!   4 bytes -> F3M-4
//!   5 bytes -> F3M-5 (prefix-stable D1-D3 + LAMBDA extension)
//! Any other length is rejected fail-closed until separately admitted.
//!
//! This module selects the physical profile only. Semantic proof status is
//! tracked separately; adding a physical width never implies production admission.

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Frame3MicroProfile {
    F3M1,
    F3M2,
    F3M3,
    F3M4,
    F3M5,
}

impl Frame3MicroProfile {
    pub const fn physical_bytes(self) -> usize {
        match self {
            Self::F3M1 => 1,
            Self::F3M2 => 2,
            Self::F3M3 => 3,
            Self::F3M4 => 4,
            Self::F3M5 => 5,
        }
    }

    pub const fn payload_bits(self) -> usize {
        self.physical_bytes() * 8
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct UnsupportedFrame3MicroLength {
    pub physical_bytes: usize,
}

pub const fn frame3_micro_profile_from_len(
    physical_bytes: usize,
) -> Result<Frame3MicroProfile, UnsupportedFrame3MicroLength> {
    match physical_bytes {
        1 => Ok(Frame3MicroProfile::F3M1),
        2 => Ok(Frame3MicroProfile::F3M2),
        3 => Ok(Frame3MicroProfile::F3M3),
        4 => Ok(Frame3MicroProfile::F3M4),
        5 => Ok(Frame3MicroProfile::F3M5),
        other => Err(UnsupportedFrame3MicroLength {
            physical_bytes: other,
        }),
    }
}

/// Select F3M profile from framing information only.
///
/// Deliberately ignores byte contents: there is no magic-byte sniffing and
/// therefore no hidden per-file marker cost.
pub fn frame3_micro_profile(
    payload: &[u8],
) -> Result<Frame3MicroProfile, UnsupportedFrame3MicroLength> {
    frame3_micro_profile_from_len(payload.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_length_selects_profile() {
        assert_eq!(frame3_micro_profile(&[0x00]), Ok(Frame3MicroProfile::F3M1));
        assert_eq!(
            frame3_micro_profile(&[0x00, 0x00]),
            Ok(Frame3MicroProfile::F3M2)
        );
        assert_eq!(
            frame3_micro_profile(&[0x00, 0x00, 0x00]),
            Ok(Frame3MicroProfile::F3M3)
        );
        assert_eq!(
            frame3_micro_profile(&[0x00, 0x00, 0x00, 0x00]),
            Ok(Frame3MicroProfile::F3M4)
        );
        assert_eq!(
            frame3_micro_profile(&[0x00, 0x00, 0x00, 0x00, 0x00]),
            Ok(Frame3MicroProfile::F3M5)
        );
    }

    #[test]
    fn byte_contents_never_select_profile() {
        for byte in 0u8..=u8::MAX {
            assert_eq!(
                frame3_micro_profile(&[byte]),
                Ok(Frame3MicroProfile::F3M1)
            );
        }
        assert_eq!(
            frame3_micro_profile(&[0xF3, 0x00]),
            Ok(Frame3MicroProfile::F3M2)
        );
        assert_eq!(
            frame3_micro_profile(&[0x00, 0xF3, 0xFF]),
            Ok(Frame3MicroProfile::F3M3)
        );
        assert_eq!(
            frame3_micro_profile(&[0xF3, 0xF4, 0x00, 0xFF]),
            Ok(Frame3MicroProfile::F3M4)
        );
        assert_eq!(
            frame3_micro_profile(&[0xF3, 0xF4, 0x00, 0xFF, 0x7A]),
            Ok(Frame3MicroProfile::F3M5)
        );
    }

    #[test]
    fn unsupported_lengths_fail_closed() {
        for len in [0usize, 6, 8, 255] {
            assert_eq!(
                frame3_micro_profile_from_len(len),
                Err(UnsupportedFrame3MicroLength {
                    physical_bytes: len,
                })
            );
        }
    }

    #[test]
    fn profile_capacity_is_full_physical_width() {
        assert_eq!(Frame3MicroProfile::F3M1.payload_bits(), 8);
        assert_eq!(Frame3MicroProfile::F3M2.payload_bits(), 16);
        assert_eq!(Frame3MicroProfile::F3M3.payload_bits(), 24);
        assert_eq!(Frame3MicroProfile::F3M4.payload_bits(), 32);
        assert_eq!(Frame3MicroProfile::F3M5.payload_bits(), 40);
    }
}
