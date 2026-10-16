//! Exact D24+ NEG/RATIO mechanical frame roundtrip (not a T5 grammar change).
//! Independent witness: semantic identity, canonicality, bit length, and
//! refusal to interpret legacy/unqualified numbers as admitted Dn values.
use sens::{
    BinaryFrame, BinaryFrameError, BinaryNumber,
    Value, PackedBitstream, decode_binary_frame, encode_binary_frame, encode_numeric_value_frame,
    decode_numeric_value_frame, encode_packed_numeric_value, decode_packed_numeric_value,
};

fn d(width: usize, v: u64) -> BinaryNumber {
    let mut bits = vec![0; width];
    for i in 0..width.min(64) { bits[width-i-1] = ((v >> i) & 1) as u8; }
    BinaryNumber::from_numeric_domain_bits(width, &bits).unwrap()
}

fn bits_to_bytes(bits: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; bits.len().div_ceil(8)];
    for (i, &bit) in bits.iter().enumerate() { out[i / 8] |= bit << (7 - i % 8); }
    out
}

fn gamma0(n: usize) -> Vec<u8> {
    let n = n + 1;
    let len = usize::BITS as usize - n.leading_zeros() as usize;
    let mut out = vec![0; len - 1];
    for i in (0..len).rev() { out.push(((n >> i) & 1) as u8); }
    out
}

#[test]
fn signed_and_rational_values_roundtrip_without_domain_loss() {
    for width in [24usize,48,96,192,384] {
        let one = Value::BinaryNumber(d(width,1));
        let two = Value::BinaryNumber(d(width,2));
        let three = Value::BinaryNumber(d(width,3));
        let neg = sens::domain_numeric_value("-",&one,&two).expect("neg");
        let ratio = sens::domain_numeric_value("/",&one,&three).expect("third");
        let neg_ratio = sens::domain_numeric_value("*",&neg,&ratio).expect("negative third");
        for value in [one, two, three, neg, ratio, neg_ratio] {
            let bits = encode_numeric_value_frame(&value).expect("framed");
            let tag = match value { Value::BinaryNumber(_) => [1,1,1,1,0,1], _ => [1,1,1,1,1,1] };
            assert_eq!(&bits[..6], &tag);
            let (recovered, consumed) = decode_numeric_value_frame(&bits).unwrap();
            assert_eq!(recovered, value);
            assert_eq!(consumed,bits.len());
            assert_eq!(encode_numeric_value_frame(&recovered).unwrap(),bits);
            let bytes = bits_to_bytes(&bits);
            assert_eq!(bytes.len(),bits.len().div_ceil(8));
            assert!(bytes.last().is_some());
            let packed = encode_packed_numeric_value(&value).unwrap();
            assert_eq!(packed.bytes(), bytes);
            assert_eq!(packed.bit_len(), bits.len());
            assert_eq!(decode_packed_numeric_value(&packed).unwrap(),value);
            let (reconstructed_bytes, reconstructed_len) = packed.clone().into_parts();
            let from_parts = PackedBitstream::from_parts(reconstructed_bytes, reconstructed_len)
                .expect("canonical padding and exact meaningful bit count");
            assert_eq!(decode_packed_numeric_value(&from_parts).unwrap(),value);
            if packed.bit_len() % 8 != 0 {
                let (mut bytes, bit_len) = packed.clone().into_parts();
                *bytes.last_mut().unwrap() |= 1; // a forbidden physical padding bit
                assert!(PackedBitstream::from_parts(bytes, bit_len).is_none());
            }
            let mut with_tail = bits.clone();
            with_tail.push(0); // extra meaningful bit must never be ignored
            let mut tail_bytes = bits_to_bytes(&with_tail);
            let tail = PackedBitstream::from_parts(std::mem::take(&mut tail_bytes),
                with_tail.len()).unwrap();
            assert!(decode_packed_numeric_value(&tail).is_err());
            let mut broken = bits.clone();
            broken.pop();
            assert!(decode_numeric_value_frame(&broken).is_err());
        }
    }
}

#[test]
fn canonical_composition_frame_rejects_negative_zero_nonreduced_and_unit_denominator() {
    for (ratio,negative,numerator,denominator) in [
        (false,true,0,0),
        (false,false,2,0),
        (true,false,2,4),
        (true,true,4,2),
        (true,false,0,2),
        (true,false,3,1),
        (true,true,3,0),
    ] {
        let mut bits = vec![1,1,1,1,1,1];
        bits.extend(gamma0(24));
        bits.push(u8::from(ratio));
        bits.push(u8::from(negative));
        bits.extend(d(24,numerator).exact_numeric_domain_bits().unwrap());
        if ratio { bits.extend(d(24,denominator).exact_numeric_domain_bits().unwrap()); }
        assert!(matches!(
            decode_binary_frame(&bits),
            Err(BinaryFrameError::NonCanonicalNumber {..})
                | Err(BinaryFrameError::InvalidNumber {..})
        ));
    }
    let mut bad_width=vec![1,1,1,1,1,1];
    bad_width.extend(gamma0(25));
    assert!(matches!(decode_binary_frame(&bad_width),
        Err(BinaryFrameError::UnsupportedDomainWidth { width:25,.. })));
}

#[test]
fn numeric_frame_bytes_cannot_be_misread_as_physical_t5_program() {
    // The existing T5 transport admits only physical bytes 0..242.
    // EXT=11 numeric records begin 111111, so their first packed byte
    // always exceeds 242 and MUST be rejected by the physical T5 reader.
    let minus_two = sens::domain_numeric_value("-",
        &Value::BinaryNumber(d(24,1)), &Value::BinaryNumber(d(24,3)))
        .expect("same D24 negative");
    let packet = encode_packed_numeric_value(&minus_two).unwrap();
    assert!(packet.bytes()[0] > 242);
    assert!(matches!(
        sens::decode_ternary_words(packet.bytes()),
        Err(sens::TernaryTransportError::InvalidPhysicalByte)
    ));
}

#[test]
fn framing_keeps_legacy_rational_and_unqualified_numeric_distinct() {
    let legacy=BinaryFrame::BinaryNumber(BinaryNumber::one());
    let enc=encode_binary_frame(&legacy).unwrap();
    assert_eq!(decode_binary_frame(&enc).unwrap().0,legacy);
    assert!(decode_numeric_value_frame(&enc).is_err());
    let natural=encode_numeric_value_frame(&Value::BinaryNumber(d(24,1))).unwrap();
    assert_ne!(natural,enc);
    assert!(encode_numeric_value_frame(&Value::BinaryNumber(BinaryNumber::one())).is_err());
    let positive=Value::BinaryNumber(d(24,1));
    assert!(encode_numeric_value_frame(&positive).is_ok());
}
