//! ZIG-OWNED: physical bit-packing is the sterile Zig corridor's job (t5/f3c/transport.zig).
//! Rust is byte-addressed and must NOT pack: Zig hands it already-packed programs.
//! This module is retained only for the staged removal (#5617/#5609). Do not extend it.
//! Draft #2189 outer-record envelope candidate S.
//!
//! This module is a **mechanical transport candidate**, not language authority.
//! It must not be wired into the canonical reader until #2189 chooses the
//! standalone envelope.
//!
//! Record prefix tree:
//!
//! ```text
//! 000..110  D1..D7 + exact payload
//! 1110      D8 carrier + exact payload
//! 11110     N + gamma0(width) + canonical binary payload
//! 111110    L + gamma0(depth) + gamma0(index)
//! 1111110   T7 + gamma0(cell_count) + 7*N payload
//! 1111111   reserved
//! ```
//!
//! Message termination is owned by the exact meaningful-bit length in
//! `OuterEnvelope`. D2 payload `11` therefore remains ordinary D2 data;
//! this codec never interprets it as a transport escape.

use crate::{
    BinaryNumber, BinarySourceWord, Bit1, Bit2, Bit3, Bit4, Bit5, Bit6, Bit7, Bit8,
    BitPacker, DomainIdentity, PackedBitstream, Text7,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OuterRecord {
    D(DomainIdentity),
    N(BinaryNumber),
    L { depth: u32, index: u32 },
    T(Text7),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OuterEnvelopeError {
    UnexpectedEnd { bit: usize },
    ReservedExtension { bit: usize },
    LengthOverflow { bit: usize },
    InvalidDomain { width: usize, payload: u16 },
    InvalidNumber { bit: usize },
    InvalidText { bit: usize },
    NonCanonicalContainer,
}

#[derive(Clone, Eq, PartialEq)]
pub struct OuterEnvelope {
    packed: PackedBitstream,
}

impl OuterEnvelope {
    pub fn from_parts(bytes: Vec<u8>, meaningful_bit_len: usize) -> Result<Self, OuterEnvelopeError> {
        let packed = PackedBitstream::from_parts(bytes, meaningful_bit_len)
            .ok_or(OuterEnvelopeError::NonCanonicalContainer)?;
        Ok(Self { packed })
    }

    pub fn bytes(&self) -> &[u8] {
        self.packed.bytes()
    }

    pub const fn meaningful_bit_len(&self) -> usize {
        self.packed.bit_len()
    }

    pub const fn valid_bits_in_last_byte(&self) -> u8 {
        self.packed.valid_bits_in_last_byte()
    }

    pub fn into_parts(self) -> (Vec<u8>, usize) {
        self.packed.into_parts()
    }

    pub fn decode(&self) -> Result<Vec<OuterRecord>, OuterEnvelopeError> {
        let mut cursor = BitCursor::new(&self.packed);
        let mut out = Vec::new();

        while cursor.position() < self.packed.bit_len() {
            out.push(decode_record(&mut cursor)?);
        }

        // Strict canonicality: every admitted byte/bit spelling must be exactly
        // the one emitted by this encoder. This is transport canonicality only.
        let replay = encode_outer_records(&out)?;
        if replay != *self {
            return Err(OuterEnvelopeError::NonCanonicalContainer);
        }

        Ok(out)
    }
}

pub fn encode_outer_records(records: &[OuterRecord]) -> Result<OuterEnvelope, OuterEnvelopeError> {
    // Stream into the existing exact bit-packer: no temporary Vec<u8>
    // carrying one host BYTE for every meaningful SENS bit.
    let mut packed = BitPacker::new();
    for record in records {
        encode_record(record, &mut packed)?;
    }
    Ok(OuterEnvelope { packed: packed.finish() })
}

fn encode_record(record: &OuterRecord, out: &mut BitPacker) -> Result<(), OuterEnvelopeError> {
    match record {
        OuterRecord::D(identity) => {
            let width = identity.width();
            let payload = identity.packed_bits();

            if !(1..=8).contains(&width) {
                return Err(OuterEnvelopeError::InvalidDomain { width, payload });
            }

            if width <= 7 {
                append_fixed((width - 1) as u64, 3, out);
            } else {
                append_fixed(0b1110, 4, out);
            }
            append_fixed(u64::from(payload), width, out);
        }
        OuterRecord::N(value) => {
            // OuterEnvelope is still a draft: its N field contains only
            // normalized magnitude bits. Explicit D24/D48/... identity
            // must never disappear during this legacy/candidate transport.
            if value.numeric_domain_width().is_some() {
                return Err(OuterEnvelopeError::InvalidNumber { bit: out.bit_len() });
            }
            append_fixed(0b11110, 5, out);
            append_gamma0(value.width(), out)?;
            for &bit in value.as_bits() {
                out.push(Bit1::new(bit).expect("BinaryNumber stores only exact bits"));
            }
        }
        OuterRecord::L { depth, index } => {
            append_fixed(0b111110, 6, out);
            append_gamma0(*depth as usize, out)?;
            append_gamma0(*index as usize, out)?;
        }
        OuterRecord::T(value) => {
            append_fixed(0b1111110, 7, out);
            append_gamma0(value.len(), out)?;
            for &cell in value.cells() {
                append_fixed(u64::from(cell), 7, out);
            }
        }
    }

    Ok(())
}

fn decode_record(cursor: &mut BitCursor<'_>) -> Result<OuterRecord, OuterEnvelopeError> {
    let start = cursor.position();
    let root = cursor.read_u8(3)?;

    if root < 0b111 {
        let width = usize::from(root) + 1;
        let payload = cursor.read_u8(width)?;
        return domain_from_parts(width, payload)
            .map(OuterRecord::D)
            .ok_or(OuterEnvelopeError::InvalidDomain { width, payload: u16::from(payload) });
    }

    // 1110 -> D8
    if cursor.read_bit()? == 0 {
        let width = 8;
        let payload = cursor.read_u8(width)?;
        return domain_from_parts(width, payload)
            .map(OuterRecord::D)
            .ok_or(OuterEnvelopeError::InvalidDomain { width, payload: u16::from(payload) });
    }

    // 11110 -> N
    if cursor.read_bit()? == 0 {
        let width = cursor.read_gamma0()?;
        if width == 0 {
            return Err(OuterEnvelopeError::InvalidNumber { bit: start });
        }
        let bits = cursor.read_vec(width)?;
        let value = BinaryNumber::from_canonical_bits(&bits)
            .map_err(|_| OuterEnvelopeError::InvalidNumber { bit: start })?;
        return Ok(OuterRecord::N(value));
    }

    // 111110 -> L
    if cursor.read_bit()? == 0 {
        let depth = cursor.read_gamma0()?;
        let index = cursor.read_gamma0()?;
        let depth = u32::try_from(depth)
            .map_err(|_| OuterEnvelopeError::LengthOverflow { bit: start })?;
        let index = u32::try_from(index)
            .map_err(|_| OuterEnvelopeError::LengthOverflow { bit: start })?;
        return Ok(OuterRecord::L { depth, index });
    }

    // 1111110 -> T7
    if cursor.read_bit()? == 0 {
        let count = cursor.read_gamma0()?;
        let needed = count
            .checked_mul(7)
            .ok_or(OuterEnvelopeError::LengthOverflow { bit: start })?;
        if cursor.remaining() < needed {
            return Err(OuterEnvelopeError::UnexpectedEnd {
                bit: cursor.position(),
            });
        }
        let mut cells = Vec::with_capacity(count.min(1 << 20));
        for _ in 0..count {
            cells.push(cursor.read_u8(7)?);
        }
        let value = Text7::from_cells(cells)
            .map_err(|_| OuterEnvelopeError::InvalidText { bit: start })?;
        return Ok(OuterRecord::T(value));
    }

    Err(OuterEnvelopeError::ReservedExtension { bit: start })
}

fn domain_from_parts(width: usize, payload: u8) -> Option<DomainIdentity> {
    let source = match width {
        1 => BinarySourceWord::W1(Bit1::new(payload)?),
        2 => BinarySourceWord::W2(Bit2::new(payload)?),
        3 => BinarySourceWord::W3(Bit3::new(payload)?),
        4 => BinarySourceWord::W4(Bit4::new(payload)?),
        5 => BinarySourceWord::W5(Bit5::new(payload)?),
        6 => BinarySourceWord::W6(Bit6::new(payload)?),
        7 => BinarySourceWord::W7(Bit7::new(payload)?),
        8 => BinarySourceWord::W8(Bit8::new(payload)?),
        _ => return None,
    };
    Some(source.domain_identity())
}

fn append_gamma0(value: usize, out: &mut BitPacker) -> Result<(), OuterEnvelopeError> {
    let encoded = value
        .checked_add(1)
        .ok_or(OuterEnvelopeError::LengthOverflow { bit: out.bit_len() })?;
    let width = usize::BITS as usize - encoded.leading_zeros() as usize;

    for _ in 0..width - 1 {
        out.push(Bit1::new(0).expect("literal bit"));
    }
    append_fixed(encoded as u64, width, out);
    Ok(())
}

fn append_fixed(value: u64, width: usize, out: &mut BitPacker) {
    for shift in (0..width).rev() {
        out.push(Bit1::new(((value >> shift) & 1) as u8).expect("masked bit"));
    }
}

// Independent old bit-for-bit oracle retained ONLY for negative/golden tests;
// production never allocates one host byte per semantic bit.
#[cfg(test)]
fn pack_bits(bits: &[u8]) -> PackedBitstream {
    let mut bytes = vec![0u8; bits.len().div_ceil(8)];

    for (index, &bit) in bits.iter().enumerate() {
        debug_assert!(bit <= 1);
        if bit == 1 {
            bytes[index / 8] |= 1 << (7 - index % 8);
        }
    }

    PackedBitstream::from_parts(bytes, bits.len())
        .expect("freshly packed exact bits always form a canonical container")
}

struct BitCursor<'a> {
    packed: &'a PackedBitstream,
    position: usize,
}

impl<'a> BitCursor<'a> {
    fn new(packed: &'a PackedBitstream) -> Self {
        Self {
            packed,
            position: 0,
        }
    }

    const fn position(&self) -> usize {
        self.position
    }

    fn remaining(&self) -> usize {
        self.packed.bit_len().saturating_sub(self.position)
    }

    fn read_bit(&mut self) -> Result<u8, OuterEnvelopeError> {
        if self.position >= self.packed.bit_len() {
            return Err(OuterEnvelopeError::UnexpectedEnd { bit: self.position });
        }

        let byte = self.packed.bytes()[self.position / 8];
        let bit = (byte >> (7 - self.position % 8)) & 1;
        self.position += 1;
        Ok(bit)
    }

    fn read_u8(&mut self, width: usize) -> Result<u8, OuterEnvelopeError> {
        debug_assert!(width <= 8);
        let mut value = 0u8;
        for _ in 0..width {
            value = (value << 1) | self.read_bit()?;
        }
        Ok(value)
    }

    fn read_vec(&mut self, width: usize) -> Result<Vec<u8>, OuterEnvelopeError> {
        if self.remaining() < width {
            return Err(OuterEnvelopeError::UnexpectedEnd { bit: self.position });
        }

        let mut bits = Vec::with_capacity(width.min(1 << 20));
        for _ in 0..width {
            bits.push(self.read_bit()?);
        }
        Ok(bits)
    }

    fn read_gamma0(&mut self) -> Result<usize, OuterEnvelopeError> {
        let start = self.position;
        let mut zeros = 0usize;
        while self.position < self.packed.bit_len() {
            if self.read_bit()? == 1 {
                break;
            }
            zeros = zeros
                .checked_add(1)
                .ok_or(OuterEnvelopeError::LengthOverflow { bit: start })?;
        }

        if self.position == self.packed.bit_len()
            && self.packed.bit_len() > 0
            && ((self.packed.bytes()[(self.position - 1) / 8]
                >> (7 - (self.position - 1) % 8))
                & 1)
                == 0
        {
            return Err(OuterEnvelopeError::UnexpectedEnd { bit: self.position });
        }

        if zeros >= usize::BITS as usize {
            return Err(OuterEnvelopeError::LengthOverflow { bit: start });
        }

        let mut encoded = 1usize;
        for _ in 0..zeros {
            let bit = usize::from(self.read_bit()?);
            encoded = encoded
                .checked_mul(2)
                .and_then(|value| value.checked_add(bit))
                .ok_or(OuterEnvelopeError::LengthOverflow { bit: start })?;
        }

        encoded
            .checked_sub(1)
            .ok_or(OuterEnvelopeError::LengthOverflow { bit: start })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(width: usize, payload: u8) -> OuterRecord {
        OuterRecord::D(domain_from_parts(width, payload).expect("valid exact domain"))
    }

    fn raw(bits: &str) -> OuterEnvelope {
        let bits = bits
            .bytes()
            .map(|byte| match byte {
                b'0' => 0,
                b'1' => 1,
                _ => panic!("raw test bits must be binary"),
            })
            .collect::<Vec<_>>();
        OuterEnvelope {
            packed: pack_bits(&bits),
        }
    }

    #[test]
    fn every_d1_through_d8_carrier_round_trips_exactly() {
        let mut count = 0usize;
        for width in 1usize..=8 {
            for payload in 0u16..(1u16 << width) {
                let record = d(width, payload as u8);
                let envelope = encode_outer_records(std::slice::from_ref(&record)).unwrap();
                assert!(envelope.decode().unwrap() == vec![record]);
                count += 1;
            }
        }
        assert_eq!(count, 510);
    }

    #[test]
    fn exact_2046_candidate_census_and_independent_byte_golden_vectors() {
        // D1..D10 has 2046 possible *coordinates*, not 2046 callable functions.
        // Candidate S is still only a D1..D8 transport proposal, not authority.
        let mut candidates = 0usize;
        let mut admitted = 0usize;
        let mut rejected = 0usize;
        let mut semantic_bits = 0usize;
        let mut batch = Vec::new();
        for width in 1u8..=10 {
            for bits in 0u16..(1u16 << width) {
                let c = crate::domain_ladder::DomainCoordinate::new(width, bits)
                    .expect("coordinate width/payload");
                candidates += 1;
                semantic_bits += usize::from(width);
                assert_eq!(c.width(), width, "first loss at D{width}:{bits:010b}");
                assert_eq!(c.bits(), bits, "first bit loss at D{width}:{bits:010b}");

                if width <= 8 {
                    let record = d(width as usize, bits as u8);
                    let packed = encode_outer_records(std::slice::from_ref(&record))
                        .expect("D1..D8 candidate S");
                    let header = if width <= 7 {
                        format!("{:03b}", width - 1)
                    } else {
                        String::from("1110")
                    };
                    let visible = format!("{header}{bits:0w$b}", w = usize::from(width));
                    let reference_bits = visible.bytes().map(|b| b - b'0').collect::<Vec<_>>();
                    let independently_packed = pack_bits(&reference_bits);
                    assert_eq!(packed.bytes(), independently_packed.bytes(),
                        "first physical byte mismatch D{width} bits={bits}");
                    assert_eq!(packed.meaningful_bit_len(), reference_bits.len(),
                        "first exact-width mismatch D{width} bits={bits}");
                    assert_eq!(packed.decode().unwrap(), vec![record.clone()],
                        "first decode identity mismatch D{width} bits={bits}");
                    batch.push(record);
                    admitted += 1;
                } else if width == 9 {
                    // D9 is valid *mechanical* source data, but the #2189
                    // prefix tree has no D9 record. Never narrow through u8.
                    let identity = BinarySourceWord::W9(
                        crate::Bit9::new(bits).expect("D9 carrier")
                    ).domain_identity();
                    let error = match encode_outer_records(&[OuterRecord::D(identity)]) {
                        Err(error) => error,
                        Ok(_) => panic!("D9:{bits:09b} falsely admitted into draft D1..D8 tree"),
                    };
                    assert_eq!(error, OuterEnvelopeError::InvalidDomain {
                        width: 9, payload: bits,
                    }, "D9:{bits:09b} was narrowed or misreported");
                    rejected += 1;
                } else {
                    // D10 exists as an owner-ratified coordinate, not as a
                    // runtime DomainIdentity or an envelope record.
                    assert_eq!(c.owner_residency(), Some(true));
                    assert_eq!(c.width(), 10);
                    assert_eq!(c.bits(), bits);
                    rejected += 1;
                }
            }
        }
        assert_eq!((candidates, admitted, rejected), (2046, 510, 1536));
        assert_eq!(semantic_bits, 18_434);
        let stream = encode_outer_records(&batch).unwrap();
        // Sum(width*2^width + prefix*2^width) for candidate S D1..D8.
        assert_eq!(stream.meaningful_bit_len(), 5_372);
        assert_eq!(stream.bytes().len(), 672);
        assert_eq!(stream.decode().unwrap(), batch,
            "cross-byte concatenation changed one of the 510 coordinates");
    }

    #[test]
    fn unratified_outer_n_path_cannot_strip_d24_numeric_identity() {
        let domain_number = BinaryNumber::from_numeric_domain_bits(24, &vec![0; 24]).unwrap();
        assert!(matches!(
            encode_outer_records(&[OuterRecord::N(domain_number)]),
            Err(OuterEnvelopeError::InvalidNumber { .. })
        ));
    }

    #[test]
    fn mixed_current_classes_round_trip_without_human_names() {
        let records = vec![
            d(2, 0b11),
            d(3, 0b001),
            OuterRecord::N(BinaryNumber::parse("101").unwrap()),
            OuterRecord::L { depth: 2, index: 7 },
            OuterRecord::T(Text7::from_cells(vec![0, 1, 127]).unwrap()),
        ];
        let envelope = encode_outer_records(&records).unwrap();
        assert!(envelope.decode().unwrap() == records);
    }

    #[test]
    fn equal_payloads_across_domains_and_number_never_collapse() {
        let d1 = encode_outer_records(&[d(1, 1)]).unwrap();
        let d3 = encode_outer_records(&[d(3, 1)]).unwrap();
        let d4 = encode_outer_records(&[d(4, 1)]).unwrap();
        let n1 = encode_outer_records(&[OuterRecord::N(BinaryNumber::one())]).unwrap();

        assert!(d1 != d3);
        assert!(d3 != d4);
        assert!(d1 != n1);
    }

    #[test]
    fn d2_dot_payload_is_data_not_transport_escape() {
        let record = d(2, 0b11);
        let envelope = encode_outer_records(std::slice::from_ref(&record)).unwrap();
        assert_eq!(envelope.meaningful_bit_len(), 5);
        assert!(envelope.decode().unwrap() == vec![record]);
    }

    #[test]
    fn candidate_s_keeps_expected_mechanical_record_costs() {
        assert_eq!(encode_outer_records(&[d(3, 0b001)]).unwrap().meaningful_bit_len(), 6);
        assert_eq!(encode_outer_records(&[d(8, 0)]).unwrap().meaningful_bit_len(), 12);

        let w16 = BinaryNumber::parse(&format!("1{}", "0".repeat(15))).unwrap();
        assert_eq!(
            encode_outer_records(&[OuterRecord::N(w16)])
                .unwrap()
                .meaningful_bit_len(),
            30
        );

        assert_eq!(
            encode_outer_records(&[OuterRecord::L { depth: 0, index: 0 }])
                .unwrap()
                .meaningful_bit_len(),
            8
        );
    }

    #[test]
    fn exact_meaningful_length_rejects_nonzero_physical_tail() {
        assert!(matches!(
            OuterEnvelope::from_parts(vec![0b1010_0001], 4),
            Err(OuterEnvelopeError::NonCanonicalContainer)
        ));
    }

    #[test]
    fn reserved_truncated_and_noncanonical_records_fail_closed() {
        assert!(matches!(
            raw("1111111").decode(),
            Err(OuterEnvelopeError::ReservedExtension { .. })
        ));
        assert!(matches!(
            raw("11110").decode(),
            Err(OuterEnvelopeError::UnexpectedEnd { .. })
        ));

        // N prefix + gamma0(2)=011 + payload 01: leading zero is non-canonical.
        assert!(matches!(
            raw("1111001101").decode(),
            Err(OuterEnvelopeError::InvalidNumber { .. })
        ));
    }

    #[test]
    fn wide_number_and_nested_local_keep_exact_coordinates() {
        let wide = BinaryNumber::parse(&format!("1{}", "01".repeat(64))).unwrap();
        let records = vec![
            OuterRecord::N(wide),
            OuterRecord::L {
                depth: u32::MAX,
                index: 1_000_000,
            },
        ];
        let envelope = encode_outer_records(&records).unwrap();
        assert!(envelope.decode().unwrap() == records);
    }
}
