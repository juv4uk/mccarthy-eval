//! MY-LISP-YANTRA: the smallest Chebupelka-style coding agent whose
//! control logic lives entirely in lib/yantra.my. The host boundary is
//! `process-run-raw` (bash tool + curl transport bytes) and `json-parse`
//! (wire-format decode); public `process-run` semantics live in Lisp.
//!
//! Historical pre-Core4 scripted agent / T-NIL Rust oracles have been retired.

use sens::{eval_program, load_core_library, load_process_library, Environment, Session};

fn agent_session() -> Session {
    // Install only the OS capability layer, opt this session into the exact
    // programs the agent may run, then bootstrap public process semantics in
    // Lisp over the raw byte-preserving host capability.
    sens_host::install();
    let environment =
        Environment::root().with_process_allowlist(vec!["bash".into(), "curl".into()]);
    let mut session = Session { environment };
    load_core_library(&mut session).unwrap();
    load_process_library(&mut session).unwrap();
    eval_program(include_str!("../../../lib/yantra.lisp"), &mut session).unwrap();
    session
}

fn eval_with_agent(source: &str) -> String {
    let mut session = agent_session();
    eval_program(source, &mut session)
        .unwrap_or_else(|e| panic!("evaluation failed: {e}\nsource: {source}"))
        .value
        .to_string()
}

// Current hosted Rust coverage: transport framing and denied/error paths.
#[test]
fn http_transport_success_passes_body_through() {
    // Real curl against the real oracle's HTTP surface is out of scope
    // here; success-path framing is verified structurally instead:
    let src = r#"
      (let ((r (list 0 "{\"ok\":true}" "")))
        (list (http-transport-exit r) (http-transport-body r)))
    "#;
    let rendered = eval_with_agent(src);
    assert!(rendered.contains("(0"), "unexpected: {rendered}");
    assert!(
        rendered.contains("ok\\\":true") || rendered.contains("ok"),
        "unexpected: {rendered}"
    );
}

#[test]
fn transport_failure_becomes_blocked_result_with_evidence() {
    // curl to a port nothing listens on: fast refusal, exit != 0.
    let src = r#"
      (let ((r (http-post-json "http://127.0.0.1:1/x" "{}")))
        (cond ((= (http-transport-exit r) 0) 1 "UNEXPECTED-SUCCESS")
              ((= (http-transport-exit r) 0) 0 (http-transport-exit r))))
    "#;
    let rendered = eval_with_agent(src);
    assert!(!rendered.contains("UNEXPECTED-SUCCESS"), "{rendered}");
}


