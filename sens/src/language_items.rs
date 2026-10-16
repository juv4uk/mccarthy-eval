//! Discoverable language items for tooling.
//!
//! Runtime builtin names come from the same root environment used by the
//! evaluator. Tooling therefore cannot silently retain a stale copy when a
//! first-class builtin is added. Syntax-dispatched forms remain explicit
//! because they are not ordinary environment bindings.
//!
//! ADR-007 peer spellings may bind the same `Value::Builtin` under several
//! human names. Metadata follows the shared builtin value, not the spelling by
//! which that value was found, so adding a peer name does not invent another
//! operation signature.

use crate::{semantic_registry::{self, LegacyRegistrySlot}, CoreDomainIdentity};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LanguageItemKind {
    Builtin,
    Macro,
    SyntaxForm,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Arity {
    Exact(usize),
    AtLeast(usize),
    Between { min: usize, max: usize },
}

impl Arity {
    pub fn accepts(self, received: usize) -> bool {
        match self {
            Self::Exact(expected) => received == expected,
            Self::AtLeast(minimum) => received >= minimum,
            Self::Between { min, max } => (min..=max).contains(&received),
        }
    }

    pub fn expected(self) -> String {
        match self {
            Self::Exact(expected) => expected.to_string(),
            Self::AtLeast(minimum) => format!("at least {minimum}"),
            Self::Between { min, max } => format!("between {min} and {max}"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LanguageItem {
    pub name: String,
    /// Canonical domain-qualified identity when this registry row has migrated.
    pub domain_identity: Option<CoreDomainIdentity>,
    pub signature: &'static str,
    pub documentation: &'static str,
    pub kind: LanguageItemKind,
    pub arity: Arity,
}

mod generated {
    include!("function_signatures_generated.rs");
}

/// Метадані для інструментів (LSP, довідка REPL) — лише за кодом СЕНС, зі
/// згенерованої проєкції lib/surface/function-signatures.lisp. Назви дає
/// таблиця функцій; Rust не тримає власної копії назв чи описів.
fn semantic_language_items_with(
    stable_surfaces: impl Fn(LegacyRegistrySlot) -> Vec<&'static str>,
    admitted_surfaces: impl Fn(LegacyRegistrySlot) -> Vec<&'static str>,
) -> Vec<LanguageItem> {
    let mut items = Vec::new();
    for row in generated::FUNCTION_SIGNATURES {
        // The generated signature table is a HISTORICAL W8 ledger, not a
        // source of current D8 or any other domain identity. It may supply
        // documentation, never the authority for a current operation.
        let semantic_id = LegacyRegistrySlot::from_packed_byte(row.semantic_id);
        let surfaces = if row.admitted_surfaces {
            admitted_surfaces(semantic_id)
        } else {
            stable_surfaces(semantic_id)
        };
        items.extend(surfaces.into_iter().map(|name| LanguageItem {
            name: name.to_string(),
            domain_identity: semantic_registry::domain_identity_for_surface(name),
            signature: row.signature,
            documentation: row.documentation,
            kind: row.kind,
            arity: row.arity,
        }));
    }
    items
}

/// Вид функції таблиці за кодом (з lib/surface/function-signatures.lisp).
/// Історичний запит: тільки для старих W8-споживачів у проєкції.
/// Поточна доменна тотожність надходить виключно з таблиці поверхонь.
pub(crate) fn signature_kind(semantic_id: LegacyRegistrySlot) -> Option<LanguageItemKind> {
    generated::FUNCTION_SIGNATURES
        .iter()
        .find(|row| row.semantic_id == semantic_id.packed_byte())
        .map(|row| row.kind)
}

pub fn language_items() -> Vec<LanguageItem> {
    semantic_language_items_with(
        semantic_registry::stable_surfaces_for_semantic_id,
        semantic_registry::admitted_surfaces_for_semantic_id,
    )
}


/// Інструментальна проєкція лише ратифікованих точних тотожностей.
///
/// Наявність рядка в старому восьмибітному реєстрі не є дозволом до
/// виконання або автоматичною тотожністю D8: навіть коли біти однакові.
/// Історичні метадані залишаються доступні через `language_items`.
#[allow(dead_code)]
pub(crate) fn current_domain_language_items() -> Vec<LanguageItem> {
    language_items()
        .into_iter()
        .filter(|item| item.domain_identity.is_some())
        .collect()
}

#[cfg(test)]
mod exact_domain_metadata_migration_tests {
    use super::*;

    #[test]
    fn admitted_canonical_function_metadata_carries_exact_domain() {
        let items = language_items();
        for name in ["як-є", "атом?", "перше", "решта", "сполучити"] {
            if let Some(row) = items.iter().find(|item| item.name == name) {
                let identity = row.domain_identity
                    .expect("ратифікована функція не може мати нетипізований metadata ID");
                assert_eq!(
                    semantic_registry::domain_identity_for_surface(name),
                    Some(identity),
                    "metadata повинні відповідати єдиній доменній таблиці"
                );
            }
        }
    }
}
