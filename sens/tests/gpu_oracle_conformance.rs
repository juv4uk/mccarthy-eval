//! GPU candidates remain blocked until SENS-owned witnesses ratify exact effects.
//! Rust does not publish CPU result meanings as GPU or language authority.
use sens::{parse_canonical_binary, syntax::ExprKind};

pub const GPU_STATUS: &str = "blocked-mechanism";

#[test]
fn d3_candidate_coordinates_are_exact_width_domain_data() {
    for payload in 0usize..8 {
        let source = format!("{payload:03b}");
        let forms = parse_canonical_binary(&source).expect("D3 coordinate parses");
        let ExprKind::DomainIdentity(identity) = &forms[0].kind else {
            panic!("candidate must remain a domain identity");
        };
        assert_eq!(identity.width(), 3);
        assert_eq!(identity.packed_bits() as usize, payload);
    }
    assert_eq!(GPU_STATUS, "blocked-mechanism");
}