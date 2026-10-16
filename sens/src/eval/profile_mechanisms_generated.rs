// GENERATED — DO NOT EDIT BY HAND.
// Authority: lib/function-table-mechanisms.lisp (profile-routes)
// Generator: scripts/generate-rust-profile-mechanism-routes.lisp

use crate::{CoreProfile, CoreDomainIdentity};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProfileMechanismRouteKind {
    RegisteredHostMechanism,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ProfileMechanismRoute {
    profile: CoreProfile,
    sens: CoreDomainIdentity,
    kind: ProfileMechanismRouteKind,
}

const PROFILE_MECHANISM_ROUTES: &[ProfileMechanismRoute] = &[
    ProfileMechanismRoute { profile: CoreProfile::Core3, sens: crate::DomainIdentity::D10(crate::DomainD10::new(0b0001010111).expect("D10 funcall")), kind: ProfileMechanismRouteKind::RegisteredHostMechanism },
];

pub(crate) fn profile_mechanism_route(
    profile: CoreProfile,
    sens: CoreDomainIdentity,
) -> Option<ProfileMechanismRouteKind> {
    PROFILE_MECHANISM_ROUTES
        .iter()
        .find(|row| row.profile == profile && row.sens == sens)
        .map(|row| row.kind)
}
