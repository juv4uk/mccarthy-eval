//! Retired Rust language-law oracle. This target now checks only the exact-width domain ladder.
use sens::{parse_canonical_binary, syntax::ExprKind};

#[test]
fn rust_observes_exact_width_domain_words_only() {
    for width in 1usize..=9 {
        let payload = (1usize << width) - 1;
        let source = format!("{payload:0width$b}");
        let forms = parse_canonical_binary(&source).expect("domain word parses");
        assert_eq!(forms.len(), 1);
        let ExprKind::DomainIdentity(identity) = &forms[0].kind else {
            panic!("binary word must remain a domain identity");
        };
        assert_eq!(identity.width(), width);
        assert_eq!(identity.packed_bits() as usize, payload);
    }
}

#[test]
fn leading_zeroes_keep_the_word_on_its_original_rung() {
    for width in 1usize..=9 {
        let source = format!("{value:0width$b}", value = 1usize);
        let forms = parse_canonical_binary(&source).expect("width-qualified word parses");
        let ExprKind::DomainIdentity(identity) = &forms[0].kind else {
            panic!("binary word must remain a domain identity");
        };
        assert_eq!(identity.width(), width);
        assert_eq!(identity.packed_bits(), 1);
    }
}