//! Runtime projection of the Lisp-owned semantic registry.
//!
//! The canonical authority is lib/surface/semantic-registry.lisp and its
//! Lisp-owned reader/API. This module contains no parser for canonical source
//! text. The generated table is emitted by
//! scripts/generate-rust-semantic-registry.lisp and is only a mechanical
//! runtime projection for fast lookup.
//
//! Generated rows may carry a packed byte as substrate representation of an
//! already understood Lisp Binary identity. This wrapper converts that byte to
//! opaque CoreDomainIdentity immediately; runtime registry APIs never expose decimal IDs.

use std::{collections::HashMap, sync::OnceLock};

use crate::{Bit3, Bit4, Bit5, Bit6, Bit9, CoreD5, CoreD6, CoreD9, CoreDomainIdentity, DomainIdentity};
use crate::CoreDomainIdentity;

mod generated {
    include!("semantic_registry_generated.rs");
}

mod domain_surface_generated {
    include!("domain_surface_registry_generated.rs");
}

mod d7_display_generated {
    include!("d7_display_registry_generated.rs");
}

mod d5_definition_bindings_generated {
    include!("d5_definition_bindings_generated.rs");
}

mod d6_definition_bindings_generated {
    include!("d6_definition_bindings_generated.rs");
}

mod d9_definition_bindings_generated {
    include!("d9_definition_bindings_generated.rs");
}

use d5_definition_bindings_generated::D5_DEFINITION_BINDINGS;
use d6_definition_bindings_generated::D6_DEFINITION_BINDINGS;
use d9_definition_bindings_generated::D9_DEFINITION_BINDINGS;
use domain_surface_generated::{DomainSurfaceRow, DOMAIN_SURFACE_ROWS};
use d7_display_generated::D7_DISPLAY_ROWS;
use generated::{SemanticRow, SEMANTIC_ROWS};

/// Only the historical W8 surface-table slot. This alias is deliberately NOT
/// called a semantic ID: D3/D4/D5/D6/D8 identities are domain-qualified,
/// and no W8 payload can be implicitly promoted to a current D8 resident.
pub(crate) type LegacyRegistrySlot = CoreDomainIdentity;

/// Convert only explicit generated D3–D6 coordinates into resident identity.
///
/// No W8 slot, packed payload, or name may select the width. Higher domains
/// require their own generated coordinate authority (never an implicit lift).
fn exact_domain_resident_from_projection(width: u8, bits: u8) -> Option<DomainIdentity> {
    use crate::BinarySourceWord;
    let source = match width {
        3 => BinarySourceWord::W3(Bit3::new(bits)?),
        4 => BinarySourceWord::W4(Bit4::new(bits)?),
        5 => BinarySourceWord::W5(Bit5::new(bits)?),
        6 => BinarySourceWord::W6(crate::Bit6::new(bits)?),
        _ => return None,
    };
    Some(DomainIdentity::from_source_word(source))
}

/// A stable source name has either exactly one domain-qualified coordinate or
/// none. Conflicts are terminal: a third equal row must not re-admit a name
/// that was already ambiguous between two widths or two bit patterns.
#[derive(Default)]
struct ExactDomainSourceIndex {
    all: HashMap<&'static str, Option<DomainIdentity>>,
    uk: HashMap<&'static str, Option<DomainIdentity>>,
}

fn insert_exact_source(
    index: &mut HashMap<&'static str, Option<DomainIdentity>>,
    name: &'static str,
    identity: Option<DomainIdentity>,
) {
    use std::collections::hash_map::Entry;
    match index.entry(name) {
        Entry::Vacant(slot) => {
            slot.insert(identity);
        }
        Entry::Occupied(mut slot) => {
            // None is a terminal collision/malformed-coordinate state.
            // A later A in A,B,A must never resurrect a callable route.
            if let Some(previous) = *slot.get() {
                if Some(previous) != identity {
                    slot.insert(None);
                }
            }
        }
    }
}

/// Build from the Lisp-owned ratified generated table only.
///
/// A malformed generated coordinate is rejected just like a collision. The
/// Ukrainian exact-source index stays separate from the language projections:
/// historical W8 and the English compatibility registry cannot authorize a
/// new executable source head.
fn build_exact_source_index(rows: &[DomainSurfaceRow]) -> ExactDomainSourceIndex {
    let mut index = ExactDomainSourceIndex::default();
    for row in rows.iter().filter(|row| row.source_routable) {
        let identity = exact_domain_resident_from_projection(row.width, row.bits);
        for surface in row.surfaces.iter() {
            if !matches!(surface.namespace, "uk" | "sa" | "en") {
                continue;
            }
            insert_exact_source(&mut index.all, surface.name, identity);
            if surface.namespace == "uk" {
                insert_exact_source(&mut index.uk, surface.name, identity);
            }
        }
    }
    index
}

fn exact_source_index() -> &'static ExactDomainSourceIndex {
    static INDEX: OnceLock<ExactDomainSourceIndex> = OnceLock::new();
    INDEX.get_or_init(|| build_exact_source_index(DOMAIN_SURFACE_ROWS))
}

/// Source human names select only the exact D3/D4/D5/D6 projection.
/// A name conflict cannot silently pick the first generated row.
fn direct_domain_identity_for_surface(name: &str) -> Option<CoreDomainIdentity> {
    exact_source_index().all.get(name).copied().flatten()?.core_operation()
}

/// Сумісний SID лише знаходить рядок surface-реєстру; координату D3
/// встановлює виключно ратифікована доменна таблиця. Немає збігу або
/// кілька різних координат — fail-closed, жодної таблиці SID→роль у Rust.
pub(crate) fn compatibility_d3_route_from_ratified_domains(
    semantic_id: LegacyRegistrySlot,
) -> Option<CoreDomainIdentity> {
    let packed = semantic_id.packed_byte();
    let row = SEMANTIC_ROWS.get(usize::from(packed))?;
    if row.semantic_id != packed {
        return None;
    }
    let mut admitted = None;
    for surface in row.surfaces {
        // Лише іменні історичні проєкції підтверджують чинний D3-закон.
        // Байт W8 та символічна мітка не мають права призначати D8/D3.
        if !matches!(surface.namespace, "ук" | "укр" | "sa") {
            continue;
        }
        let Some(identity) = direct_domain_identity_for_surface(surface.name) else {
            continue;
        };
        if !matches!(identity, CoreDomainIdentity::D3(_)) {
            return None;
        }
        if admitted.is_some_and(|previous| previous != identity) {
            return None;
        }
        admitted = Some(identity);
    }
    admitted
}

/// Canonical Ukrainian *source-head* projection, before evaluation.
///
/// Only owner-ratified source-routable uk rows are eligible. This is NOT the
/// legacy SID compatibility index: no en alias, old W8, D2 structure, or
/// unratified D7 binder can become a callable head via this API.
/// The mixed reader applies it only to executable HEAD positions.
pub(crate) fn exact_uk_callable_for_source_head(name: &str) -> Option<crate::DomainIdentity> {
    exact_source_index().uk.get(name).copied().flatten()
}


/// Binding-only OD-005 bootstrap projection for Lisp-owned definitions.
///
/// This MUST NOT be used to reinterpret historical CoreDomainIdentity calls. Its only
/// consumer is `bind_language_definition`: a definition discovered through
/// the compatibility surface registry is bound once into an already-ratified
/// exact D5 slot. #3062 removes this bootstrap detour.
/// Binding-only exact D5 lookup for existing Lisp definitions.
///
/// The table is generated from Lisp-owned projection data and may only supply
/// a mechanism to an identity already ratified by #3305/#3331.
pub(crate) fn d5_binding_identity_for_definition(name: &str) -> Option<CoreDomainIdentity> {
    D5_DEFINITION_BINDINGS
        .iter()
        .find(|row| row.name == name)
        .map(|row| CoreDomainIdentity::D5(CoreD5::from_word(Bit5::new(row.bits).unwrap())))
}

/// Binding-only D6 lookup for an already-evaluated Lisp definition.
///
/// A matching name does not make D6 callable. The caller may populate an
/// exact mechanism slot only when a real root callable value exists.
pub(crate) fn d6_binding_identity_for_definition(name: &str) -> Option<DomainIdentity> {
    D6_DEFINITION_BINDINGS
        .iter()
        .find(|row| row.names.iter().any(|candidate| *candidate == name))
        .map(|row| {
            DomainIdentity::D6(CoreD6::from_word(
                Bit6::new(row.bits).expect("generated D6 binding is exactly six bits"),
            ))
        })
}

/// Binding-only D9 lookup for language-owned Lisp definitions.
/// D9 residency remains separate from callability: only a concrete evaluated
/// definition can populate the exact mechanism slot.
pub(crate) fn d9_binding_identity_for_definition(name: &str) -> Option<DomainIdentity> {
    D9_DEFINITION_BINDINGS
        .iter()
        .find(|row| row.names.iter().any(|candidate| *candidate == name))
        .map(|row| DomainIdentity::D9(CoreD9::from_word(
            Bit9::new(row.bits).expect("generated D9 binding is exactly nine bits"),
        )))
}

/// Канонічний surface → domain маршрут читає тільки ратифіковану
/// exact-width проєкцію. Історичний SID/байт не визначає домен.
pub(crate) fn domain_identity_for_surface(name: &str) -> Option<CoreDomainIdentity> {
    direct_domain_identity_for_surface(name)
}

/// D7 Ukrainian keyboard projection for the physical T5 text boundary.
/// No semantic role is inferred from equal bits in D2/another width.
pub(crate) fn d7_uk_keyboard_glyph(cell: u8) -> Option<&'static str> {
    D7_DISPLAY_ROWS
        .iter()
        .find(|row| row.bits == cell)
        .and_then(|row| row.uk)
}

pub(crate) fn d7_uk_keyboard_cell(glyph: &str) -> Option<u8> {
    D7_DISPLAY_ROWS
        .iter()
        .find(|row| row.uk == Some(glyph))
        .map(|row| row.bits)
}

pub(crate) fn surface_for_domain_identity(
    identity: DomainIdentity,
    namespace: &str,
) -> Option<&'static str> {
    // D7 is a sound/text *display* projection only. Its exact W7 coordinate
    // is already known; no reverse name lookup or callable status is inferred.
    if matches!(identity, DomainIdentity::D7(_)) {
        let row = D7_DISPLAY_ROWS.iter().find(|row|
            u16::from(row.bits) == identity.packed_bits()
        )?;
        return match namespace {
            "uk" => row.uk,
            "sa" => row.sa,
            "en" => row.en,
            "en-ipa" => row.en_ipa,
            _ => None,
        };
    }

    DOMAIN_SURFACE_ROWS
        .iter()
        .find(|row| {
            usize::from(row.width) == identity.width()
                && u16::from(row.bits) == identity.packed_bits()
        })?
        .surfaces
        .iter()
        .find_map(|surface| (surface.namespace == namespace).then_some(surface.name))
}
pub(crate) fn semantic_id_bits(semantic_id: LegacyRegistrySlot) -> String {
    semantic_id.to_string()
}

fn live_rows() -> &'static [SemanticRow] {
    SEMANTIC_ROWS
}



pub(crate) fn admitted_semantic_ids() -> Vec<LegacyRegistrySlot> {
    live_rows()
        .iter()
        .map(|row| CoreDomainIdentity::from_packed_byte(row.semantic_id))
        .collect()
}

fn insert_surface_mapping(
    index: &mut HashMap<&'static str, LegacyRegistrySlot>,
    surface: &'static str,
    semantic_id: LegacyRegistrySlot,
) {
    if let Some(previous) = index.insert(surface, semantic_id) {
        if previous != semantic_id {
            panic!(
                "generated semantic registry surface must be unique: {surface} maps to both {} and {}",
                semantic_id_bits(previous),
                semantic_id_bits(semantic_id)
            );
        }
    }
}

fn surface_index() -> &'static HashMap<&'static str, LegacyRegistrySlot> {
    static INDEX: OnceLock<HashMap<&'static str, LegacyRegistrySlot>> = OnceLock::new();
    INDEX.get_or_init(|| {
        let mut index = HashMap::new();
        for row in live_rows() {
            for surface in row.surfaces {
                insert_surface_mapping(&mut index, surface.name, CoreDomainIdentity::from_packed_byte(row.semantic_id));
            }
        }
        index
    })
}

pub(crate) fn admitted_semantic_id_for_surface(name: &str) -> Option<LegacyRegistrySlot> {
    surface_index().get(name).copied()
}

fn stable_surfaces_from_index(
    index: &HashMap<&'static str, LegacyRegistrySlot>,
    semantic_id: LegacyRegistrySlot,
) -> Vec<&'static str> {
    let mut surfaces = index
        .iter()
        .filter_map(|(surface, mapped_id)| (*mapped_id == semantic_id).then_some(*surface))
        .collect::<Vec<_>>();
    surfaces.sort_unstable();
    surfaces
}

fn admitted_surfaces_from_rows(
    rows: &[SemanticRow],
    semantic_id: LegacyRegistrySlot,
) -> Vec<&'static str> {
    let mut surfaces = rows
        .iter()
        .find(|row| row.semantic_id == semantic_id.packed_byte())
        .into_iter()
        .flat_map(|row| row.surfaces.iter().map(|surface| surface.name))
        .collect::<Vec<_>>();
    surfaces.sort_unstable();
    surfaces
}

pub(crate) fn semantic_id_for_surface(name: &str) -> Option<LegacyRegistrySlot> {
    admitted_semantic_id_for_surface(name)
}

pub(crate) fn stable_surfaces_for_semantic_id(
    semantic_id: LegacyRegistrySlot,
) -> Vec<&'static str> {
    stable_surfaces_from_index(surface_index(), semantic_id)
}

pub(crate) fn admitted_surfaces_for_semantic_id(
    semantic_id: LegacyRegistrySlot,
) -> Vec<&'static str> {
    admitted_surfaces_from_rows(live_rows(), semantic_id)
}

pub(crate) fn admitted_surfaces_with_namespace_for_semantic_id(
    semantic_id: LegacyRegistrySlot,
) -> Vec<(&'static str, &'static str)> {
    let mut surfaces = live_rows()
        .iter()
        .find(|row| row.semantic_id == semantic_id.packed_byte())
        .into_iter()
        .flat_map(|row| row.surfaces.iter().map(|s| (s.namespace, s.name)))
        .collect::<Vec<_>>();
    surfaces.sort_unstable();
    surfaces
}

#[cfg(test)]
mod exact_registry_vs_historical_slot_tests {
    use super::*;
    use crate::{Bit8, CoreD8};

    #[test]
    fn historical_w8_slot_cannot_become_current_d8_round() {
        let historical: LegacyRegistrySlot = crate::DomainIdentity::D10(crate::DomainD10::new(0b0001010111).expect("D10 funcall"));
        let current = DomainIdentity::D8(
            CoreD8::from_word(Bit8::new(0b10101000).unwrap()),
        );
        // Bit-for-bit equality of the mechanical payload is insufficient.
        assert_eq!(historical.packed_byte() as u16, current.packed_bits());
        assert_eq!(current.width(), 8);
        assert_eq!(current.to_string(), "10101000");
        // D8 is ratified in the Lisp table but has NO callable source
        // route in this generated runtime surface projection.
        assert_eq!(surface_for_domain_identity(current, "en"), None);
    }

    #[test]
    fn surface_domain_projection_never_requires_the_legacy_byte_slot() {
        for (surface, width, bits) in [
            ("атом?", 3usize, 0b010u16),
            ("як-є", 3usize, 0b001u16),
        ] {
            let identity = domain_identity_for_surface(surface)
                .expect("generated ratified D3 surface must have an exact domain");
            assert_eq!(identity.width(), width);
            assert_eq!(u16::from(identity.packed_bits()), bits);
        }
        assert_eq!(domain_identity_for_surface("__unknown_domain_name__"), None);
    }
}

#[cfg(test)]
mod exact_d6_binding_projection_tests {
    use super::*;

    #[test]
    fn reduce_definition_resolves_to_exact_d6_without_generic_callability() {
        let identity = d6_binding_identity_for_definition("reduce")
            .expect("ratified D6 REDUCE must have binding-only identity");
        assert_eq!(identity.width(), 6);
        assert_eq!(identity.packed_bits(), 0b101110);
        assert!(identity.core_operation().is_none(),
            "binding authority must not grant generic D6 callability");
    }

    #[test]
    fn unknown_definition_does_not_gain_d6_identity() {
        assert_eq!(d6_binding_identity_for_definition("__unknown_d6_binding__"), None);
    }
}

#[cfg(test)]
mod exact_d9_binding_projection_tests {
    use super::*;

    #[test]
    fn reasoning_definitions_have_exact_d9_binding_identity_only() {
        for (name, bits) in [
            ("unify", 0b101010000u16),
            ("apply-subst", 0b101101001u16),
            ("var?", 0b101100100u16),
            ("walk", 0b101110100u16),
        ] {
            let identity = d9_binding_identity_for_definition(name).expect("ratified D9 binding");
            assert_eq!(identity.packed_bits(), bits);
            assert!(identity.core_operation().is_none());
        }
    }
}

#[cfg(test)]
mod exact_d5_binding_projection_tests {
    use super::*;

    #[test]
    fn every_generated_d5_definition_uses_its_exact_domain_word() {
        for row in D5_DEFINITION_BINDINGS {
            let identity = d5_binding_identity_for_definition(row.name)
                .expect("generated Lisp-owned D5 binding must resolve");
            assert_eq!(
                identity,
                CoreDomainIdentity::D5(CoreD5::from_word(
                    Bit5::new(row.bits).expect("generated D5 bit width")
                )),
                "the canonical D5 identity must come from the generated ladder row"
            );
        }
    }

    #[test]
    fn unknown_definition_does_not_gain_a_d5_coordinate() {
        assert_eq!(d5_binding_identity_for_definition("__unknown_d5_binding__"), None);
    }
}


#[cfg(test)]
mod exact_d3_compatibility_migration_tests {
    use super::*;

    #[test]
    fn only_ratified_surfaces_project_historical_quote_cond_car() {
        for (legacy_byte, bits) in [
            (0b00000001, 0b001u8),
            (0b00000111, 0b110u8),
            (0b00000101, 0b100u8),
        ] {
            let identity = compatibility_d3_route_from_ratified_domains(
                CoreDomainIdentity::from_packed_byte(legacy_byte)
            ).expect("іменна історична проєкція на ратифікований D3");
            assert_eq!(identity.width(), 3);
            assert_eq!(identity.packed_bits(), bits);
        }
    }

    #[test]
    fn unrelated_w8_never_becomes_d3() {
        for byte in [0b00000000, 0b00001000, 0b00001001, 0b10101000] {
            assert!(compatibility_d3_route_from_ratified_domains(
                CoreDomainIdentity::from_packed_byte(byte)
            ).is_none());
        }
    }
}

#[cfg(test)]
mod domain_source_projection_anti_collision_tests {
    use super::*;
    use super::domain_surface_generated::DomainSurfaceName;

    static SHARED_UK: [DomainSurfaceName; 1] = [DomainSurfaceName {
        namespace: "uk",
        name: "однакова-назва",
    }];
    static AMBIGUOUS_SANSKRIT: [DomainSurfaceName; 1] = [DomainSurfaceName {
        namespace: "sa",
        name: "eka",
    }];

    #[test]
    fn different_d3_and_d4_coordinates_with_same_name_fail_closed_in_either_order() {
        let d3 = DomainSurfaceRow {
            width: 3, bits: 0b001, source_routable: true, surfaces: &SHARED_UK,
        };
        let d4 = DomainSurfaceRow {
            width: 4, bits: 0b0001, source_routable: true, surfaces: &SHARED_UK,
        };
        // The low numeric payload is equal. The domain widths are not.
        for rows in [[d3, d4, d3], [d4, d3, d4]] {
            let index = build_exact_source_index(&rows);
            assert_eq!(index.all.get("однакова-назва"), Some(&None));
            assert_eq!(index.uk.get("однакова-назва"), Some(&None));
        }
    }

    #[test]
    fn invalid_width_or_payload_cannot_become_a_callable_d3_word() {
        let invalid_width = DomainSurfaceRow {
            width: 8, bits: 0b001, source_routable: true, surfaces: &SHARED_UK,
        };
        let invalid_payload = DomainSurfaceRow {
            width: 3, bits: 0b1000, source_routable: true, surfaces: &AMBIGUOUS_SANSKRIT,
        };
        let index = build_exact_source_index(&[invalid_width, invalid_payload]);
        assert_eq!(index.all.get("однакова-назва"), Some(&None));
        assert_eq!(index.all.get("eka"), Some(&None));
        assert_eq!(index.uk.get("однакова-назва"), Some(&None));
        assert!(!index.uk.contains_key("eka"));
    }

    #[test]
    fn every_ratified_source_row_keeps_exact_width_and_bit_identity() {
        let index = build_exact_source_index(DOMAIN_SURFACE_ROWS);
        let mut rows_checked = 0usize;
        let mut surfaces_checked = 0usize;
        for row in DOMAIN_SURFACE_ROWS.iter().filter(|row| row.source_routable) {
            rows_checked += 1;
            let expected = exact_domain_resident_from_projection(row.width, row.bits)
                .expect("source-routable D3–D6 row must have a valid exact word");
            assert_eq!(expected.width(), usize::from(row.width));
            assert_eq!(expected.packed_bits(), u16::from(row.bits));
            for surface in row.surfaces.iter() {
                if !matches!(surface.namespace, "uk" | "sa" | "en") {
                    continue;
                }
                surfaces_checked += 1;
                assert_eq!(
                    index.all.get(surface.name).copied().flatten(),
                    Some(expected),
                    "a generated surface lost its exact domain identity"
                );
                if surface.namespace == "uk" {
                    assert_eq!(index.uk.get(surface.name).copied().flatten(), Some(expected));
                } else {
                    assert!(!index.uk.contains_key(surface.name));
                }
                assert_eq!(
                    domain_identity_for_surface(surface.name),
                    expected.core_operation(),
                    "only admitted domain mechanisms may be callable"
                );
            }
        }
        assert_eq!(rows_checked, 119);
        assert_eq!(surfaces_checked, 238);
    }

    #[test]
    fn all_2046_coordinates_and_120_exact_d3_d6_slots_preserve_their_width() {
        use std::collections::HashSet;

        let mut unique = HashSet::new();
        let mut exact_routable_widths = 0usize;
        for width in 1u8..=10 {
            for bits in 0..(1u16 << width) {
                let coordinate = crate::domain_ladder::DomainCoordinate::new(width, bits)
                    .expect("власні біти кожного домену");
                assert_eq!(coordinate.width(), width);
                assert_eq!(coordinate.bits(), bits);
                assert!(unique.insert(coordinate), "два домени злилися в одну координату");

                if width > 1 && bits < (1u16 << (width - 1)) {
                    let narrower = crate::domain_ladder::DomainCoordinate::new(width - 1, bits)
                        .expect("той самий payload у вужчому домені");
                    assert_ne!(coordinate, narrower);
                }

                // Source-callability is narrower than the 2046 mechanical
                // positions. It is *not* inferred from occupancy or payload.
                if (3..=6).contains(&width) {
                    let identity = exact_domain_resident_from_projection(width, bits as u8)
                        .expect("кожен механічний D3–D6 слот має точну форму");
                    assert_eq!(identity.width(), usize::from(width));
                    assert_eq!(identity.packed_bits(), bits);
                    exact_routable_widths += 1;
                } else if bits < 256 {
                    assert!(
                        exact_domain_resident_from_projection(width, bits as u8).is_none(),
                        "поза D3–D6 не можна створювати callable source head"
                    );
                }
            }
        }
        assert_eq!(unique.len(), 2046, "2+4+8+...+1024 координат");
        assert_eq!(exact_routable_widths, 120, "8+16+32+64 D3–D6 слотів");
    }

    #[test]
    fn all_120_d3_d6_collisions_remain_denied_after_repetition_and_reordering() {
        // Input permutations A,B,A and B,A,B probe the REAL registry insert
        // state machine, not a mock parser. A duplicate after a conflict is
        // not a new law and may not restore a route.
        for width in 3u8..=6 {
            for bits in 0..(1u16 << width) {
                let a = exact_domain_resident_from_projection(width, bits as u8).unwrap();
                let other_width = if width == 6 { 3 } else { width + 1 };
                let other_bits = (bits & ((1u16 << other_width) - 1)) as u8;
                let b = exact_domain_resident_from_projection(other_width, other_bits).unwrap();
                assert_ne!(a, b);

                for sequence in [[a, b, a, b], [b, a, b, a]] {
                    let mut index = HashMap::new();
                    for (step, identity) in sequence.into_iter().enumerate() {
                        insert_exact_source(&mut index, "суперечлива-назва", Some(identity));
                        if step > 0 {
                            assert_eq!(
                                index.get("суперечлива-назва"), Some(&None),
                                "колізія {width}:{bits} відновилася на кроці {step}"
                            );
                        }
                    }
                }

                let mut identical = HashMap::new();
                for _ in 0..4 {
                    insert_exact_source(&mut identical, "одна-координата", Some(a));
                }
                assert_eq!(identical.get("одна-координата"), Some(&Some(a)));

                let mut malformed = HashMap::new();
                insert_exact_source(&mut malformed, "неправильна-координата", None);
                insert_exact_source(&mut malformed, "неправильна-координата", Some(a));
                assert_eq!(malformed.get("неправильна-координата"), Some(&None));
            }
        }
    }

    #[test]
    fn never_route_d1_d2_or_historical_w8_from_domain_surface_index() {
        let index = exact_source_index();
        for name in ["ні", "так", "відкрити", "закрити", "пропуск", "крапка"] {
            assert!(!index.all.contains_key(name));
            assert_eq!(direct_domain_identity_for_surface(name), None);
        }
        assert_eq!(direct_domain_identity_for_surface("def"), None);
        assert_eq!(direct_domain_identity_for_surface("00000001"), None);
        assert_eq!(direct_domain_identity_for_surface("атом?").map(|d| d.width()), Some(3));
        assert_eq!(direct_domain_identity_for_surface("як-є").map(|d| d.packed_bits()), Some(0b001));
    }
}
