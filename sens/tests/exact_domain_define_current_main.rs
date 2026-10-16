//! Mechanical exact-width coordinate tests. Source grammar has reserved words,
//! so it must not be used as a generic enumerator for every D1-D10 payload.
use sens::domain_ladder::DomainCoordinate;

#[test]
fn rust_observes_exact_width_domain_words_only() {
    for width in 1u8..=10 {
        for payload in 0u16..(1u16 << width) {
            let coordinate = DomainCoordinate::new(width, payload)
                .expect("every in-range bit word has an exact-width coordinate");
            assert_eq!(coordinate.width(), width);
            assert_eq!(coordinate.bits(), payload);
        }
        assert_eq!(
            DomainCoordinate::new(width, 1u16 << width),
            None,
            "overflow must not be truncated to the current rung"
        );
    }
}

#[test]
fn leading_zeroes_keep_the_word_on_its_original_rung() {
    for width in 1u8..=10 {
        let coordinate = DomainCoordinate::new(width, 1)
            .expect("leading-zero coordinate stays on the declared rung");
        assert_eq!(coordinate.width(), width);
        assert_eq!(coordinate.bits(), 1);
    }

    // Matching payloads never erase domain-width identity.
    assert_ne!(
        DomainCoordinate::new(3, 0b011),
        DomainCoordinate::new(4, 0b0011)
    );
    assert_ne!(
        DomainCoordinate::new(8, 0b1010_1000),
        DomainCoordinate::new(10, 0b00_1010_1000)
    );
}

#[test]
fn width_bounds_fail_closed_without_u8_or_u16_guessing() {
    assert_eq!(DomainCoordinate::new(0, 0), None);
    assert_eq!(DomainCoordinate::new(11, 0), None);
    assert_eq!(DomainCoordinate::new(8, 256), None);
    assert_eq!(DomainCoordinate::new(10, 1024), None);
    assert_eq!(DomainCoordinate::new(10, 1023).unwrap().bits(), 1023);
}
