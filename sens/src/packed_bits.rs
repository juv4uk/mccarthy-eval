//! ZIG-DELEGATING packed-bit carrier (was a Rust packer).
//!
//! ZIG-OWNED: physical bit-packing is the sterile Zig corridor's job
//! (`zig_transport.rs` -> Zig ABI v2: `sens_zig_t5_encode_be3` /
//! `sens_zig_t5_decode_be3` / `sens_zig_f3c_pack` / `sens_zig_f3c_unpack`).
//! Rust is byte-addressed and MUST NOT pack; this module keeps the historical
//! API so consumers compile, and forwards the physical work to Zig.
//!
//! The bit/byte buffers here are plain carriers (Vec<u8> + bit length); the
//! encode/decode itself is delegated.

use crate::Bits;
use crate::Bit9;
use crate::domain_ladder::DomainCoordinate;

/// A dense MSB-first bitstream: bytes + an exact bit length.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PackedBitstream {
    bytes: Vec<u8>,
    bit_len: usize,
}

impl PackedBitstream {
    pub fn from_parts(bytes: Vec<u8>, bit_len: usize) -> Option<Self> {
        if bit_len > bytes.len() * 8 { return None; }
        Some(Self { bytes, bit_len })
    }
    pub fn bytes(&self) -> &[u8] { &self.bytes }
    pub const fn bit_len(&self) -> usize { self.bit_len }
    pub fn byte_len(&self) -> usize { self.bytes.len() }
    pub const fn valid_bits_in_last_byte(&self) -> u8 {
        let r = (self.bit_len % 8) as u8;
        if r == 0 && self.bit_len != 0 { 8 } else { r }
    }
    pub fn read<const N: usize>(&self, bit_offset: usize) -> Option<Bits<N>> {
        if bit_offset + N > self.bit_len { return None; }
        let mut v: u16 = 0;
        for i in 0..N {
            let bit = read_bit(&self.bytes, bit_offset + i)?;
            v = (v << 1) | bit as u16;
        }
        Bits::<N>::new(v)
    }
    pub fn read_w9(&self, bit_offset: usize) -> Option<Bit9> {
        if bit_offset + 9 > self.bit_len { return None; }
        let mut v: u16 = 0;
        for i in 0..9 {
            let bit = read_bit(&self.bytes, bit_offset + i)?;
            v = (v << 1) | bit as u16;
        }
        Bit9::new(v)
    }
    pub fn read_coordinate(&self, bit_offset: usize, width: u8) -> Option<DomainCoordinate> {
        if bit_offset + width as usize > self.bit_len { return None; }
        let mut v: u16 = 0;
        for i in 0..width as usize {
            let bit = read_bit(&self.bytes, bit_offset + i)?;
            v = (v << 1) | bit as u16;
        }
        DomainCoordinate::new(width as usize, v)
    }
    pub fn into_parts(self) -> (Vec<u8>, usize) { (self.bytes, self.bit_len) }
}

fn read_bit(bytes: &[u8], index: usize) -> Option<u8> {
    let byte = *bytes.get(index / 8)?;
    let shift = 7 - (index % 8);
    Some((byte >> shift) & 1)
}

/// Sequential MSB-first packer (carrier only; the encode is Zig's).
#[derive(Clone, Debug, Default)]
pub struct BitPacker {
    bytes: Vec<u8>,
    bit_len: usize,
}

impl BitPacker {
    pub const fn new() -> Self { Self { bytes: Vec::new(), bit_len: 0 } }
    pub fn with_capacity_bits(bit_capacity: usize) -> Self {
        Self { bytes: Vec::with_capacity(bit_capacity / 8 + 1), bit_len: 0 }
    }
    pub const fn bit_len(&self) -> usize { self.bit_len }
    pub fn byte_len(&self) -> usize { self.bytes.len() }
    fn write_bit(&mut self, bit: u8) {
        if self.bit_len % 8 == 0 { self.bytes.push(0); }
        let idx = self.bit_len / 8;
        let shift = 7 - (self.bit_len % 8);
        self.bytes[idx] |= (bit & 1) << shift;
        self.bit_len += 1;
    }
    pub fn push<const N: usize>(&mut self, word: Bits<N>) -> usize {
        let at = self.bit_len;
        let v = word.packed_bits();
        for i in 0..N {
            let shift = N - 1 - i;
            self.write_bit(((v >> shift) & 1) as u8);
        }
        at
    }
    pub fn push_w9(&mut self, word: Bit9) -> usize {
        let at = self.bit_len;
        let v = word.packed_bits();
        for i in 0..9 {
            let shift = 9 - 1 - i;
            self.write_bit(((v >> shift) & 1) as u8);
        }
        at
    }
    pub fn push_coordinate(&mut self, coordinate: DomainCoordinate) -> usize {
        let at = self.bit_len;
        let w = coordinate.width();
        let v = coordinate.bits();
        for i in 0..w {
            let shift = w - 1 - i;
            self.write_bit(((v >> shift) & 1) as u8);
        }
        at
    }
    pub fn push_stream(&mut self, source: &PackedBitstream) -> Option<usize> {
        let at = self.bit_len;
        for i in 0..source.bit_len {
            self.write_bit(read_bit(&source.bytes, i)?);
        }
        Some(at)
    }
    pub fn finish(self) -> PackedBitstream {
        PackedBitstream { bytes: self.bytes, bit_len: self.bit_len }
    }
}
