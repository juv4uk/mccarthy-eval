//! Незалежні тести ратифікованої механічної драбини D1–D10.
//! Тотожність = (ширина, біти); виконуваність тут навмисно відсутня.

use sens::{parse_binary_source_words, parse_domain_ladder_source_words, ErrorKind};

#[test]
fn всі_1024_координати_d10_не_згортаються_у_восьмибітний_носій() {
    let words = (0..1024u16)
        .map(|bits| format!("{bits:010b}"))
        .collect::<Vec<_>>()
        .join(" ");
    let result = parse_domain_ladder_source_words(&words).expect("ратифіковані D10 координати");
    assert_eq!(result.len(), 1024);
    for (bits, token) in result.iter().enumerate() {
        assert_eq!(token.coordinate.width(), 10);
        assert_eq!(token.coordinate.bits(), bits as u16);
        assert_eq!(token.coordinate.owner_residency(), Some(true));
        assert_eq!(token.span.end - token.span.start, 10);
        assert_eq!(&words[token.span.start..token.span.end],
            format!("{bits:010b}").as_str());
    }
    // Старий carrier D1–D9 цю саму D10 форму читати НЕ повинен.
    assert!(parse_binary_source_words("0000000001").is_err());
}

#[test]
fn однаковий_payload_різної_ширини_не_є_однією_функцією() {
    let result = parse_domain_ladder_source_words(
        "1 01 001 0001 00000001 000000001 0000000001",
    ).unwrap();
    assert_eq!(
        result.iter().map(|v| v.coordinate.width()).collect::<Vec<_>>(),
        [1, 2, 3, 4, 8, 9, 10]
    );
    assert!(result.iter().all(|v| v.coordinate.bits() == 1));
    for pair in result.windows(2) {
        assert_ne!(pair[0].coordinate, pair[1].coordinate);
    }
}

#[test]
fn бінарні_слова_мають_точні_байтові_span_після_коментарів() {
    let source = "001 ; пояснення українською\n 0000000001\t10 ; коментар\n";
    let result = parse_domain_ladder_source_words(source).unwrap();
    assert_eq!(result.len(), 3);
    assert_eq!(
        result.iter().map(|x| x.coordinate.width()).collect::<Vec<_>>(),
        [3, 10, 2]
    );
    for token in result {
        let spelling = &source[token.span.start..token.span.end];
        assert_eq!(spelling.len(), token.coordinate.width() as usize);
        let mut bits = 0u16;
        for ch in spelling.bytes() {
            bits = (bits << 1) | u16::from(ch - b'0');
        }
        assert_eq!(bits, token.coordinate.bits());
    }
}

#[test]
fn небінарні_й_надширокі_джерела_блокуються_без_здогадок() {
    for source in [
        "00000000000", "000000000000000000000000", "10000000001", 
        "001 10101010101", "12", "#b1010", "001 X", "(001)", "1/2",
    ] {
        let error = parse_domain_ladder_source_words(source)
            .expect_err("це не D1–D10 координата");
        assert_eq!(error.kind, ErrorKind::Parse, "{source}");
    }
    assert!(parse_domain_ladder_source_words(" ; лише коментар\n ").unwrap().is_empty());
}
