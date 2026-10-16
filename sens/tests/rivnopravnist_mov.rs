use sens::{eval_program, Session, Value};
use std::collections::HashSet;

const REPL_КАТАЛОГ: &str = include_str!("../../sens-cli/src/repl/surface_catalog.rs");
const ПЕРЕВІРКА_ПОКРИТТЯ: &str = include_str!("../../../scripts/check_surface_coverage.py");
const ПЕРЕВІРКА_РІВНОПРАВЯ: &str = include_str!("../../../scripts/check_trilingual_surface.py");
const ТРАНСЛЯТОР: &str = include_str!("../../../scripts/translate-program.py");

fn registry_ids() -> Vec<u8> {
    sens::semantic_registry_export::admitted_semantic_ids()
}

fn registry_surfaces(
    id: impl sens::semantic_registry_export::ProjectionSidInput,
) -> Vec<sens::semantic_registry_export::SurfaceRow> {
    sens::semantic_registry_export::admitted_surfaces_for_semantic_id(id)
}

fn registry_id_bits(
    id: impl sens::semantic_registry_export::ProjectionSidInput,
) -> String {
    sens::semantic_registry_export::semantic_id_bits(id)
}

#[test]
fn семантичні_ідентифікатори_складаються_тільки_з_цифр() {
    let ids = registry_ids();
    let mut seen = HashSet::new();

    for id in &ids {
        let bits = registry_id_bits(*id);
        assert!(
            bits.len() == 8 && bits.bytes().all(|byte| matches!(byte, b'0' | b'1')),
            "SID {bits:?} порушує canonical semantic registry"
        );
        assert!(seen.insert(*id), "дубль ID {bits}");
    }

    assert_eq!(
        seen.len(),
        256,
        "canonical semantic registry має містити Canon 0 + 255 identities"
    );
}

#[test]
fn кожна_тотожність_явно_описує_uk_en_sa_без_заборони_майбутніх_мов() {
    for id in registry_ids() {
        if registry_id_bits(id) == "00000000" {
            continue;
        }

        let surfaces = registry_surfaces(id);
        let namespaces = surfaces
            .iter()
            .map(|row| row.namespace)
            .collect::<HashSet<_>>();

        assert!(
            namespaces
                .iter()
                .all(|namespace| ["en", "ук", "укр", "sa", "sym"].contains(namespace)),
            "{}: unexpected surface namespace set: {namespaces:?}",
            registry_id_bits(id)
        );
        assert_eq!(
            namespaces.len(),
            surfaces.len(),
            "{}: duplicate surface namespace",
            registry_id_bits(id)
        );
    }
}

#[test]
fn символічна_нотація_не_належить_людській_мові() {
    for id in registry_ids() {
        if registry_id_bits(id) == "00000000" {
            continue;
        }

        for row in registry_surfaces(id) {
            if row.namespace == "sym" {
                continue;
            }

            assert!(
                row.name.chars().any(char::is_alphabetic),
                "{}/{:?}: символіка, а не людська мова",
                registry_id_bits(id),
                row.name
            );
        }
    }
}

#[test]
fn додавання_відділяє_людські_мови_від_спільного_двійкового_коду() {
    let id = sens::semantic_registry_export::semantic_id_for_admitted_surface("додати")
        .expect("додати must be admitted");
    assert_eq!(registry_id_bits(id), "00001100");

    let rows = registry_surfaces(id);
    let surface = |namespace: &str| {
        rows.iter()
            .find(|row| row.namespace == namespace)
            .map(|row| row.name)
    };

    assert_eq!(surface("ук"), Some("додати"));
    assert_eq!(surface("en"), Some("plus"));
    assert_eq!(surface("sa"), Some("yoga"));
    assert_eq!(surface("sym"), Some("+"));

    let en_surface = surface("en").expect("English plus surface");
    let en_call = format!("({en_surface} 20 22)");

    let mut сесія = Session::default();
    for вираз in [
        "(додати 20 22)",
        "(+ 20 22)",
        "(yoga 20 22)",
        en_call.as_str(),
    ] {
        assert_eq!(
            eval_program(вираз, &mut сесія).unwrap().value.to_string(),
            "42"
        );
    }

    assert!(
        eval_program(en_surface, &mut сесія).is_err(),
        "bare human surface must not become Function8 during evaluation"
    );

    let exact = eval_program("00001100", &mut сесія)
        .expect("bare exact Function8 remains a binary value")
        .value;
    assert_eq!(exact, Value::Sid(sens::sens!(00001100)));
}
#[test]
fn executable_authority_більше_не_читає_legacy_en_shaped_таблицю() {
    for (імя, джерело) in [
        ("REPL", REPL_КАТАЛОГ),
        ("coverage", ПЕРЕВІРКА_ПОКРИТТЯ),
        ("parity", ПЕРЕВІРКА_РІВНОПРАВЯ),
        ("translator", ТРАНСЛЯТОР),
    ] {
        assert!(
            !джерело.contains("uk-sa-coverage.lisp"),
            "{імя}: legacy EN-shaped table знову стала executable authority"
        );
    }

    assert!(
        sens::semantic_registry_export::semantic_id_for_admitted_surface("uk-sa-coverage.lisp")
            .is_none(),
        "legacy table name must never become a semantic surface"
    );
}

#[test]
#[ignore = "фінальний gate: увімкнути після завершення UK/EN/SA parity"]
fn повне_рівноправя_вимагає_наявності_для_всіх_людських_поверхонь() {
    for id in registry_ids() {
        if registry_id_bits(id) == "00000000" {
            continue;
        }

        let rows = registry_surfaces(id);
        let present = ["ук", "en", "sa"]
            .iter()
            .all(|namespace| rows.iter().any(|row| row.namespace == *namespace));

        assert!(
            present,
            "{}: UK/EN/SA ще не заповнені одночасно",
            registry_id_bits(id)
        );
    }
}
