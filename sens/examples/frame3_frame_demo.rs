//! Демонстрація: одна проста програма -> транспортні трити -> байти кожної рамки.
//! Показує саме те, як Т5 має роздільник 2 між словами, а Рамка-3 (F3) його прибирає.
use sens::{
    decode_ternary_words, encode_ternary_words, parse_binary_source_words, BinarySourceWord,
};
use sens::universal_frame3::{decode, decode_v2, encode, encode_v2};
use sens::frame3_domain::{encode as encode_v3, decode as decode_v3, Word};

#[path = "../../../research/framed3/rust_codec/senc.rs"]
mod senc;

/// Транспортні трити T5: роздільник 2 між усіма словами + хвостове заповнення.
fn t5_trits(words: &[String]) -> Vec<u8> {
    let mut trits = Vec::new();
    for (position, word) in words.iter().enumerate() {
        if position != 0 {
            trits.push(2);
        }
        for byte in word.bytes() {
            trits.push(byte - b'0');
        }
    }
    while trits.len() % 5 != 0 {
        trits.push(2);
    }
    trits
}

/// Транспортні трити Рамки-3: `2` лише між ВНУТРІШНІМИ словами (краї злиті).
fn f3_trits(words: &[String]) -> Vec<u8> {
    let last = words.len().saturating_sub(1);
    let mut trits = Vec::new();
    for (position, word) in words.iter().enumerate() {
        if position >= 2 && position < last {
            trits.push(2);
        }
        for byte in word.bytes() {
            trits.push(byte - b'0');
        }
    }
    trits
}

fn show_trits(trits: &[u8]) -> String {
    trits.iter().map(|t| t.to_string()).collect::<Vec<_>>().join(" ")
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" ")
}

fn demo(name: &str, source: &str) {
    let words: Vec<BinarySourceWord> = parse_binary_source_words(source)
        .expect("exact source word corpus")
        .into_iter()
        .map(|item| item.word)
        .collect();
    let strings: Vec<String> = words
        .iter()
        .map(|w| {
            let packed = w.packed_bits();
            let width = w.width();
            (0..width).rev().map(|i| if (packed >> i) & 1 == 1 { '1' } else { '0' }).collect()
        })
        .collect();

    let t5 = encode_ternary_words(&words).unwrap();
    let v1 = encode(&words).unwrap();
    let v2 = encode_v2(&words).unwrap();
    let v3_words: Vec<Word> = words.iter().map(|w| Word::Core(w.clone())).collect();
    let v3 = encode_v3(&v3_words).unwrap();
    let f3_body = senc::frame_encode(&strings).unwrap();
    let mut f3 = vec![senc::FRAME_LABEL];
    f3.extend_from_slice(&f3_body);

    // Кожна рамка мусить повернути ту саму програму.
    assert_eq!(decode_ternary_words(&t5).unwrap(), words);
    assert_eq!(decode(&v1).unwrap(), words);
    assert_eq!(decode_v2(&v2).unwrap(), words);
    assert_eq!(decode_v3(&v3).unwrap(), v3_words);
    assert_eq!(senc::decode_senc(&f3).unwrap(), strings);

    println!("=== {name} ===");
    println!("  слова        : {}", strings.join(" "));
    println!("  T5  трити    : {}  ({} трит)", show_trits(&t5_trits(&strings)), t5_trits(&strings).len());
    println!("  F3  трити    : {}  ({} трит)", show_trits(&f3_trits(&strings)), f3_trits(&strings).len());
    println!("  T5  байти    : {} ({} Б)", hex(&t5), t5.len());
    println!("  S3/v1 байти   : {} ({} Б)", hex(&v1), v1.len());
    println!("  S3/v2 байти   : {} ({} Б)", hex(&v2), v2.len());
    println!("  S3/v3 байти   : {} ({} Б)", hex(&v3), v3.len());
    println!("  F3   байти    : {} ({} Б)", hex(&f3), f3.len());
    println!();
}

fn main() {
    demo("порожня структура", "10 01");
    demo("один атом 0", "10 0 01");
    demo("один атом 1", "10 1 01");
    demo("два атоми, є транспортний 2", "10 0 2 0 01");
    demo("багато форм (рамка-3 не приймає)", "10 0 01 10 1 01");
}
