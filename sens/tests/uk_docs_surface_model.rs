const README: &str = include_str!("../../../README.md");
const UKRAINIAN_API: &str = include_str!("../../../docs/ukrainian-api.md");
const DOCS_INDEX: &str = include_str!("../../../lib/surface/uk-docs.lisp");

#[test]
fn readme_explains_the_two_ukrainian_surfaces() {
    for required in [
        "`ук` — коротка, інтуїтивно зрозуміла українська поверхня",
        "`укр` — повна українська поверхня",
        "docs/generated/function-table.md",
    ] {
        assert!(README.contains(required), "README missing Ukrainian surface contract: {required}");
    }
}

#[test]
fn ukrainian_api_distinguishes_surface_names_from_function_meaning() {
    for required in [
        "`ук` — коротке ім'я",
        "`укр` — повне ім'я",
        "byte SID",
        "generated/function-table.md",
    ] {
        assert!(
            UKRAINIAN_API.contains(required),
            "docs/ukrainian-api.md missing ук/укр documentation rule: {required}"
        );
    }
}

#[test]
fn machine_docs_index_states_that_registry_owns_surface_spellings() {
    for required in [
        "byte SID",
        "ук/укр",
        "semantic-registry.lisp",
        "опис функції не дублюється",
    ] {
        assert!(
            DOCS_INDEX.contains(required),
            "lib/surface/uk-docs.lisp missing authority note: {required}"
        );
    }
}

fn documented_ids() -> Vec<String> {
    DOCS_INDEX
        .lines()
        .filter_map(|line| {
            let line = line.trim_start();
            let rest = line.strip_prefix("(doc ")?;
            let mut fields = rest.split_whitespace();
            fields.next()?;
            let id = fields.next()?;
            // A bare 8-bit SENS token; a quoted "00001100" is text, not SENS.
            let bits = id;
            (bits.len() == 8 && bits.bytes().all(|byte| matches!(byte, b'0' | b'1')))
                .then(|| bits.to_string())
        })
        .collect()
}

#[test]
fn every_documented_identity_shows_uk_ukr_and_one_behavior_description() {
    let (_, reference_and_tail) = UKRAINIAN_API
        .split_once("## Повний довідник")
        .expect("Ukrainian API must contain the detailed reference section");
    let (reference, _) = reference_and_tail
        .split_once("## Межа довідника")
        .expect("Ukrainian API detailed reference must end before the boundary section");

    assert!(
        reference.contains("| byte SID | `ук` | `укр` | Виклик | Тип | Що робить | Основа |"),
        "detailed Ukrainian API must expose ук and укр side by side"
    );

    let ids = documented_ids();
    assert!(!ids.is_empty(), "machine docs index must document semantic identities");

    let rows = reference
        .lines()
        .filter(|line| line.starts_with("| `") && line.chars().filter(|character| *character == '|').count() == 8)
        .collect::<Vec<_>>();
    assert_eq!(
        rows.len(),
        ids.len(),
        "detailed reference must contain exactly one row per documented byte SID"
    );

    for id in ids {
        let marker = format!("| `{id}` |");
        let count = rows.iter().filter(|row| row.starts_with(&marker)).count();
        assert_eq!(
            count, 1,
            "byte SID {id} must appear exactly once in the detailed ук/укр reference"
        );
    }

    for required_row_fragment in [
        "| `00111100` | `текст-порожній?` | `порожній-текст?` |",
        "| `01011010` | `монотонний-нс` | `монотонний-час-у-наносекундах` |",
    ] {
        assert!(
            reference.contains(required_row_fragment),
            "detailed reference missing representative ук/укр row: {required_row_fragment}"
        );
    }
}
