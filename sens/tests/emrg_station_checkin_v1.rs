use sens::{eval_program, Session};

const SOURCE: &str =
    include_str!("../../../fixtures/emrg/v1/station-checkin-v1.source.lisp");
const WIRE: &str =
    include_str!("../../../fixtures/emrg/v1/station-checkin-v1.wire");
const SURFACES: &str =
    include_str!("../../../fixtures/emrg/v1/station-checkin-v1.surfaces.tsv");

const EXPECTED_WIRE_HEX: &str =
    "282371323a312f31202371323a302f3120226d2d30303031222022323032362d31302d30345430303a30303a30305a2220282371323a302f312920282371323a302f312929";

fn eval(source: &str) -> String {
    let mut session = Session::default();
    eval_program(source, &mut session)
        .expect("EMRG fixture expression should evaluate")
        .value
        .to_string()
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(DIGITS[(byte >> 4) as usize] as char);
        out.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    out
}

#[test]
fn station_checkin_v1_has_runtime_verified_canonical_wire() {
    let program = format!("(write-to-string (quote {SOURCE}))");
    assert_eq!(eval(&program), format!("{WIRE:?}"));

    let round_trip = format!(
        "(write-to-string (read (write-to-string (quote {SOURCE}))))"
    );
    assert_eq!(eval(&round_trip), format!("{WIRE:?}"));
}

#[test]
fn station_checkin_v1_records_exact_consumer_payload_size() {
    assert_eq!(WIRE.len(), 69);
    assert_eq!(WIRE.len() * 8, 552);
    assert_eq!(hex(WIRE.as_bytes()), EXPECTED_WIRE_HEX);
}

#[test]
fn station_checkin_v1_wire_contains_no_human_surface_keys() {
    for forbidden in [
        "station",
        "check-in",
        "location",
        "unknown",
        "note",
        "реєстрація",
        "станції",
        "місцезнаходження",
        "невідомо",
        "zgłoszenie",
        "lokalizacja",
        "nieznana",
    ] {
        assert!(
            !WIRE.to_lowercase().contains(&forbidden.to_lowercase()),
            "human surface leaked into canonical wire: {forbidden}"
        );
    }
}

#[test]
fn station_checkin_v1_exports_three_projection_surfaces_without_wire_authority() {
    let rows: Vec<&str> = SURFACES.lines().collect();
    assert_eq!(rows[0], "path\tuk\tpl\ten");
    assert_eq!(rows.len(), 8);

    for required in [
        "Реєстрація станції",
        "Zgłoszenie stacji",
        "Station check-in",
        "Невідомо",
        "Nieznana",
        "Unknown",
        "Відсутня",
        "Brak",
        "Absent",
    ] {
        assert!(SURFACES.contains(required));
        assert!(!WIRE.contains(required));
    }
}
