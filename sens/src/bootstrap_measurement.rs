//! Test-only internal bootstrap decomposition for #3648.
//!
//! One ignored dispatch test is reused for every stage. A one-byte numeric
//! SENS_BOOTSTRAP_MEASURE_LEVEL selects the prefix depth, avoiding variable-cost
//! string dispatch inside the measured Cachegrind path.
//!
//! This module deliberately lives behind cfg(test) so private bootstrap
//! mechanisms stay private.

use super::*;
use std::hint::black_box;

fn root_session() -> Session {
    Session {
        environment: Environment::root(),
    }
}

fn prepare_profile(session: &mut Session) {
    session.environment.select_core_profile(CoreProfile::Core4);
}

fn load_first_macro(session: &mut Session) {
    load_macro_library(session).expect("macro library must load");
}

fn decode_current_core() -> Vec<Expr> {
    let (expressions, source_hash) =
        fasl_decode_program(CORE_LIBRARY_FASL).expect("embedded Core FASL must decode");
    assert_eq!(
        source_hash,
        sha256_source(CORE_LIBRARY_SOURCE.as_bytes()),
        "embedded Core FASL must match embedded Core source"
    );
    expressions
}


fn measurement_level() -> u8 {
    let raw = std::env::var("SENS_BOOTSTRAP_MEASURE_LEVEL")
        .expect("SENS_BOOTSTRAP_MEASURE_LEVEL must select a diagnostic prefix level");
    let bytes = raw.as_bytes();
    assert_eq!(
        bytes.len(),
        1,
        "SENS_BOOTSTRAP_MEASURE_LEVEL must be exactly one ASCII digit"
    );
    let byte = bytes[0];
    assert!(
        (b'0'..=b'6').contains(&byte),
        "SENS_BOOTSTRAP_MEASURE_LEVEL must be in 0..=6"
    );
    byte - b'0'
}

#[test]
#[ignore = "diagnostic benchmark for #3648"]
fn bootstrap_measure_dispatch() {
    let level = measurement_level();

    // Level 6 is the independent production-loader control. It is compared
    // with the staged path, but is deliberately not part of its monotonic prefix.
    if level == 6 {
        let mut session = root_session();
        let result = load_core_library(&mut session).expect("full Core loader must succeed");
        black_box(result);
        black_box(session);
        return;
    }

    // Levels 0..5 are intentionally one sequential prefix. Every later level
    // executes all earlier setup before adding exactly one named stage.
    let mut session = root_session();
    if level == 0 {
        black_box(session);
        return;
    }

    prepare_profile(&mut session);
    if level == 1 {
        black_box(session);
        return;
    }

    load_first_macro(&mut session);
    if level == 2 {
        black_box(session);
        return;
    }

    let expressions = decode_current_core();
    if level == 3 {
        black_box(session);
        black_box(expressions);
        return;
    }

    let result = eval_parsed_expressions(&expressions, &mut session)
        .expect("decoded Core must evaluate");
    if level == 4 {
        black_box(result);
        black_box(session);
        return;
    }

    debug_assert_eq!(level, 5);
    bind_missing_stable_surface_peers(&session.environment);
    black_box(result);
    black_box(session);
}
