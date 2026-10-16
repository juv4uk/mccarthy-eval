//! Immutable routing for evaluator mechanisms necessary beyond Canon 0 + McCarthy7.
//!
//! Stable human/symbolic spellings are resolved by the shared semantic registry.
//! Lisp owns the SID-to-operation mapping in the `(form ...)` field of
//! `lib/surface/function-signatures.lisp`.
//! This module only projects the selected operation class onto Rust evaluator mechanisms.

use crate::semantic_registry::{self, LegacyRegistrySlot};
use crate::CoreDomainIdentity;
#[cfg(test)]
use crate::{Bit4, CoreD4};

mod generated {
    include!("necessary_forms_generated.rs");
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum NecessaryFormIdentity {
    Define,
    Lambda,
}

/// A historical Lisp-signature mechanism may point only to an already
/// ratified D4 surface; its old W8 byte never supplies the current bits.
fn canonical_domain_identity(
    mechanism: generated::NecessaryFormMechanism,
) -> Option<CoreDomainIdentity> {
    let surface = match mechanism {
        generated::NecessaryFormMechanism::Lambda => "функція",
        generated::NecessaryFormMechanism::Define => "визначити",
    };
    let identity = semantic_registry::domain_identity_for_surface(surface)?;
    identity_for_domain_identity(identity).map(|_| identity)
}

/// Select the evaluator mechanism from the exact canonical domain identity.
///
/// Only D4 LAMBDA=0010 and DEFINE=0011 are necessary forms. Equal packed
/// payloads in D3/D5/D6 do not inherit this meaning.
pub(crate) fn identity_for_domain_identity(
    identity: CoreDomainIdentity,
) -> Option<NecessaryFormIdentity> {
    let CoreDomainIdentity::D4(word) = identity else {
        return None;
    };
    match word.word().packed_bits() {
        0b0010 => Some(NecessaryFormIdentity::Lambda),
        0b0011 => Some(NecessaryFormIdentity::Define),
        _ => None,
    }
}

/// Canonical domain identity for one necessary-form surface.
///
/// Stable lambda/define spellings arrive from the domain-qualified registry.
/// The historical `def` surface is compatibility-only: its legacy row is used
/// solely to recover the already-known Define mechanism and is normalized
/// immediately to canonical D4:0011. No numeric truncation is permitted.
pub(crate) fn domain_identity_for_symbol(name: &str) -> Option<CoreDomainIdentity> {
    if let Some(identity) = semantic_registry::domain_identity_for_surface(name) {
        return identity_for_domain_identity(identity).map(|_| identity);
    }

    let legacy = semantic_registry::admitted_semantic_id_for_surface(name)?;
    domain_identity_for_legacy_semantic_id(legacy)
}

/// Explicit compatibility seam: an admitted historical W8 row may identify
/// a necessary-form mechanism only by projecting it to its ratified D4 word.
/// The historical byte itself never becomes a current domain identity.
fn domain_identity_for_legacy_semantic_id(
    legacy: LegacyRegistrySlot,
) -> Option<CoreDomainIdentity> {
    // The W8 slot locates historical source surfaces, never a D4 code.
    // Stable Ukrainian/Sanskrit names must prove a unique current D4 route.
    let surfaces = semantic_registry::admitted_surfaces_with_namespace_for_semantic_id(legacy);
    let mut admitted = None;
    let mut has_qualified_surface = false;
    for (namespace, surface) in &surfaces {
        if !matches!(*namespace, "ук" | "укр" | "sa") {
            continue;
        }
        has_qualified_surface = true;
        let identity = semantic_registry::domain_identity_for_surface(surface)?;
        identity_for_domain_identity(identity)?;
        if admitted.is_some_and(|previous| previous != identity) {
            return None;
        }
        admitted = Some(identity);
    }
    if has_qualified_surface {
        return admitted;
    }

    // Only the Lisp-owned, English-only historical alias (currently "def")
    // can use its explicit signature record. Never derive a D4 route from
    // an arbitrary W8 byte or bypass a conflicting native surface.
    if surfaces.is_empty() || surfaces.iter().any(|(namespace, _)| *namespace != "en") {
        return None;
    }
    let mechanism = generated::NECESSARY_FORM_DISPATCH
        .iter()
        .find(|row| row.semantic_id == legacy.packed_byte())
        .map(|row| row.mechanism)?;
    canonical_domain_identity(mechanism)
}

pub(crate) fn identity_for_semantic_id(
    semantic_id: LegacyRegistrySlot,
) -> Option<NecessaryFormIdentity> {
    domain_identity_for_legacy_semantic_id(semantic_id)
        .and_then(identity_for_domain_identity)
}

/// Resolve an executable list-head symbol through the shared authority
/// registry, then select the evaluator mechanism by exact SID identity.
/// Uses the admitted (stable OR compatibility-only) surface index, not the
/// stable-only one `canon.rs`/tooling use elsewhere: `def`'s row is
/// compatibility-only, and dispatch must still see it as Define through
/// the registry rather than through a hardcoded `"def"` literal
/// (previously duplicated in both `eval/mod.rs` and `ir.rs` for exactly
/// this reason -- both removed once this function could see it).
pub(crate) fn identity_for_symbol(name: &str) -> Option<NecessaryFormIdentity> {
    domain_identity_for_symbol(name).and_then(identity_for_domain_identity)
}


#[cfg(test)]
mod exact_d4_necessary_form_migration_tests {
    use super::*;

    fn d4(bits: u8) -> CoreDomainIdentity {
        CoreDomainIdentity::D4(
            CoreD4::from_word(Bit4::new(bits).expect("test D4 coordinate must fit")),
        )
    }

    #[test]
    fn canonical_lambda_and_define_route_only_through_exact_d4() {
        assert_eq!(
            identity_for_domain_identity(d4(0b0010)),
            Some(NecessaryFormIdentity::Lambda),
        );
        assert_eq!(
            identity_for_domain_identity(d4(0b0011)),
            Some(NecessaryFormIdentity::Define),
        );
        assert_eq!(identity_for_domain_identity(d4(0b0001)), None);
        assert_eq!(identity_for_domain_identity(d4(0b1111)), None);
    }

    #[test]
    fn compatibility_w8_rows_are_explicitly_projected_to_d4() {
        for (slot, bits, expected) in [
            (0b00001000, 0b0010, NecessaryFormIdentity::Lambda),
            (0b00001001, 0b0011, NecessaryFormIdentity::Define),
            (0b00001011, 0b0011, NecessaryFormIdentity::Define),
        ] {
            let legacy = LegacyRegistrySlot::from_packed_byte(slot);
            let identity = domain_identity_for_legacy_semantic_id(legacy)
                .expect("the three admitted compatibility rows have an explicit D4 bridge");
            assert_eq!(identity.width(), 4);
            assert_eq!(identity.packed_bits(), bits);
            assert_eq!(identity_for_semantic_id(legacy), Some(expected));
        }
    }

    #[test]
    fn equal_payload_in_d3_does_not_inherit_d4_lambda() {
        let d3_atom = semantic_registry::domain_identity_for_surface("атом?")
            .expect("the canonical D3 atom surface is admitted");
        let d4_lambda = d4(0b0010);

        assert_eq!(d3_atom.width(), 3);
        assert_eq!(d4_lambda.width(), 4);
        assert_eq!(
            u16::from(d3_atom.packed_bits()),
            u16::from(d4_lambda.packed_bits()),
            "D3:010 and D4:0010 share a numeric payload only",
        );
        assert_eq!(identity_for_domain_identity(d3_atom), None);
        assert_eq!(
            identity_for_domain_identity(d4_lambda),
            Some(NecessaryFormIdentity::Lambda),
        );
    }

    #[test]
    fn every_other_historical_w8_slot_fails_closed() {
        for slot in 0u8..=u8::MAX {
            let expected = match slot {
                0b00001000 => Some(NecessaryFormIdentity::Lambda),
                0b00001001 | 0b00001011 => Some(NecessaryFormIdentity::Define),
                _ => None,
            };
            assert_eq!(
                identity_for_semantic_id(LegacyRegistrySlot::from_packed_byte(slot)),
                expected,
                "unexpected necessary-form route for historical W8 {slot:08b}",
            );
        }
    }
}
