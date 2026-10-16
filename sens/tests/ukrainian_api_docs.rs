const DOCS_INDEX: &str = include_str!("../../../lib/surface/uk-docs.lisp");
const UK_SURFACE: &str = include_str!("../../../lib/surface/uk.lisp");

// vsi_stable_ukrainski_nazvy_maiut_numeric_zapys_u_dovidnyku,
// dokumentatsiinyi_kliuch_ie_tilky_numeric,
// znak_pytannia_tochno_vidpovidaie_predykatam, and
// znak_oklyku_tochno_vidpovidaie_mutatsii were pure registry/doc-sync
// text checks, relocated to `cargo xtask verify` per TEST-ARCHITECTURE-1
// step 4 — see crates/xtask/src/checks.rs.

fn documented_kind(name: &str) -> Option<String> {
    let signature_prefix = format!("\"({name}");
    DOCS_INDEX.lines().find_map(|line| {
        let fields = line.split_whitespace().collect::<Vec<_>>();
        if fields.first() != Some(&"(doc") {
            return None;
        }
        assert!(
            fields.len() >= 5,
            "рядок документації має містити category numeric-ID kind signature: {line}"
        );
        line.contains(&signature_prefix)
            .then(|| fields[3].to_string())
    })
}

#[test]
fn stari_nazvy_dvokh_predykativ_lyshaiutsia_aliasamy_symisnosti() {
    // Визначення пишеться кодом СЕНС 00001001 (або старим іменем define).
    for binding in [
        "конфлікт? check-conflict)",
        "перевірити-конфлікт check-conflict)",
        "змінна-зустрічається? occurs-check)",
        "перевірити-зустрічання occurs-check)",
    ] {
        assert!(
            UK_SURFACE.contains(&format!("(00001001 {binding}"))
                || UK_SURFACE.contains(&format!("(define {binding}")),
            "відсутній compatibility alias: {binding}"
        );
    }
}

#[test]
fn dovidnyk_poiasniuie_ne_predykaty_shcho_mozhut_povernuty_pustyi_spysok() {
    for name in ["отримати-з-карти", "підтримувальний-доказ", "та", "або"] {
        let kind = documented_kind(name)
            .unwrap_or_else(|| panic!("немає документаційного запису для {name}"));
        assert_ne!(kind, "predicate");
        assert!(!name.ends_with('?'));
    }
}

// smyslovyi_audyt_pokryvaie_vsi_140_stable_nazv,
// seredovyshche_ne_maie_povtornoho_surface_binding, and
// stari_nazvy_smystovoho_audytu_lyshaiutsia_aliasamy_sumisnosti were pure
// registry/doc-sync text checks, relocated to `cargo xtask verify` per
// TEST-ARCHITECTURE-1 step 4 — see crates/xtask/src/checks.rs.
