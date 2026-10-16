use sens::{parse_binary_source_words, DomainIdentity};
use serde_json::Value;

const CERTIFICATE: &str =
    include_str!("../../../knowledge/domain-width-authority.generated.json");

#[test]
fn generated_sens_width_certificate_matches_exact_rust_carriers() {
    let certificate: Value = serde_json::from_str(CERTIFICATE).expect("valid width certificate");
    assert_eq!(
        certificate["status"],
        "generated-projection-non-authoritative",
        "Rust must consume a projection, never become width authority"
    );

    let current = certificate["current_domains"]
        .as_array()
        .expect("current_domains array");
    let domains = certificate["domains"]
        .as_object()
        .expect("domains object");

    assert_eq!(current.len(), domains.len());

    for domain in current {
        let domain = domain.as_str().expect("domain name string");
        let width = domains[domain]["width"]
            .as_u64()
            .expect("positive exact width") as usize;
        assert!(width > 0);

        // The spelling length comes from the SENS-generated certificate.
        // Rust only proves that its exact source carrier preserves that width.
        let source = "0".repeat(width);
        let tokens = parse_binary_source_words(&source).expect("certified width must parse");
        assert_eq!(tokens.len(), 1);

        let word = tokens[0].word;
        assert_eq!(
            word.width(),
            width,
            "{domain}: source carrier drifted from SENS width certificate"
        );

        let identity = DomainIdentity::from_source_word(word);
        assert_eq!(
            identity.width(),
            width,
            "{domain}: DomainIdentity drifted from SENS width certificate"
        );
    }
}
