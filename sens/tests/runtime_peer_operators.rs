use sens::{language_items, render_value_for_presentation, PresentationLanguage, Value};

const UK_SURFACE: &str = include_str!("../../../lib/surface/uk.lisp");
const SA_SURFACE: &str = include_str!("../../../lib/surface/sa.lisp");
const PRESENTATION: &str = include_str!("../src/presentation.rs");

struct PeerCase {
    uk: &'static str,
    sa: &'static str,
    sym: &'static str,
}

const CASES: &[PeerCase] = &[
    PeerCase {
        uk: "відняти",
        sa: "viyoga",
        sym: "-",
    },
    PeerCase {
        uk: "помножити",
        sa: "guṇana",
        sym: "*",
    },
    PeerCase {
        uk: "поділити",
        sa: "haraṇa",
        sym: "/",
    },
    PeerCase {
        uk: "менше?",
        sa: "hīna?",
        sym: "<",
    },
    PeerCase {
        uk: "більше?",
        sa: "adhika?",
        sym: ">",
    },
    PeerCase {
        uk: "рівне?",
        sa: "sama?",
        sym: "=",
    },
];

#[test]
fn migrated_surface_files_do_not_build_stable_operator_peers_through_symbols() {
    for forbidden in [
        "(define відняти -)",
        "(define помножити *)",
        "(define поділити /)",
        "(define менше? <)",
        "(define більше? >)",
        "(define рівне? =)",
    ] {
        assert!(!UK_SURFACE.contains(forbidden), "UK alias leaked back: {forbidden}");
    }
    for forbidden in [
        "(define viyoga -)",
        "(define guṇana *)",
        "(define haraṇa /)",
        "(define hīna? <)",
        "(define adhika? >)",
        "(define sama? =)",
    ] {
        assert!(!SA_SURFACE.contains(forbidden), "SA alias leaked back: {forbidden}");
    }
}

#[test]
fn runtime_peer_slice_matches_numeric_registry_rows() {
    for case in CASES {
        let sid = sens::semantic_registry_export::semantic_id_for_admitted_surface(case.uk)
            .expect("UK peer must be registry-admitted");
        assert_eq!(
            sens::semantic_registry_export::semantic_id_for_admitted_surface(case.sa),
            Some(sid)
        );
        assert_eq!(
            sens::semantic_registry_export::semantic_id_for_admitted_surface(case.sym),
            Some(sid)
        );
        let surfaces = sens::semantic_registry_export::admitted_surfaces_for_semantic_id(sid);
        assert!(surfaces.iter().any(|row| row.namespace == "ук" && row.name == case.uk));
        assert!(surfaces.iter().any(|row| row.namespace == "sa" && row.name == case.sa));
        assert!(surfaces.iter().any(|row| row.namespace == "sym" && row.name == case.sym));
    }
}

#[test]
fn українське_подання_розрізняє_історичний_w8_і_домени() {
    assert!(!PRESENTATION.contains("uk-sa-coverage.lisp"));
    for case in CASES {
        let sid = sens::semantic_registry_export::semantic_id_for_admitted_surface(case.sym)
            .expect("symbol peer must be registry-admitted");
        let exact = Value::Sid(sid);
        assert_eq!(
            render_value_for_presentation(&exact, PresentationLanguage::Ukrainian),
            format!("#<сумісний W8:{} {}>", sid, case.uk),
            "{} presentation",
            case.uk
        );
    }
}

#[test]
fn tooling_metadata_follows_the_shared_builtin_value_for_every_peer() {
    let items = language_items();
    for case in CASES {
        let lookup = |name: &str| {
            items
                .iter()
                .find(|item| item.name == name)
                .unwrap_or_else(|| panic!("missing tooling item {name}"))
        };
        let uk = lookup(case.uk);
        let sa = lookup(case.sa);
        let sym = lookup(case.sym);
        assert_eq!(uk.signature, sym.signature, "{} UK signature", case.uk);
        assert_eq!(sa.signature, sym.signature, "{} SA signature", case.uk);
        assert_eq!(uk.documentation, sym.documentation, "{} UK docs", case.uk);
        assert_eq!(sa.documentation, sym.documentation, "{} SA docs", case.uk);
        assert_eq!(uk.arity, sym.arity, "{} UK arity", case.uk);
        assert_eq!(sa.arity, sym.arity, "{} SA arity", case.uk);
    }
}
