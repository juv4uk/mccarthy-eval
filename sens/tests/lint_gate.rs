use sens::{
    eval_parsed_expressions, eval_pinned_legacy_core_source, eval_program,
    parse_mixed_exact_domain, parse_mixed_exact_domain_fs_source,
    parse_mixed_exact_domain_utf8_source, Session,
};
use std::fs;

/// `read-file` is language-owned (lib/fs.lisp) over the host's
/// `read-file-bytes`; installed here via the dev-dependency plus fs.lisp.
fn install_read_capability() {
    sens_host::install();
}

#[test]
fn linter_gate() {
    install_read_capability();
    let mut session = Session::default();

    // Evaluate the libraries directly to load them into the environment
    let core_src = fs::read_to_string("../../lib/core.lisp").unwrap();
    eval_pinned_legacy_core_source("lib/core.lisp", &core_src, &mut session)
        .expect("pin legacy Core source to exact domain reader");
    let utf8_src = fs::read_to_string("../../lib/utf8.lisp").unwrap();
    let utf8_forms = parse_mixed_exact_domain_utf8_source("lib/utf8.lisp", &utf8_src)
        .expect("pin UTF8 source to exact domain reader");
    eval_parsed_expressions(&utf8_forms, &mut session).expect("load UTF8");
    let fs_src = fs::read_to_string("../../lib/fs.lisp").unwrap();
    let fs_forms = parse_mixed_exact_domain_fs_source("lib/fs.lisp", &fs_src)
        .expect("pin FS source to exact domain reader");
    eval_parsed_expressions(&fs_forms, &mut session).expect("load FS");
    let linter_src = fs::read_to_string("../../lib/linter.lisp").unwrap();
    // The linter owns exact D3/D4/D6 heads.  The historical eval_program
    // reader treats e.g. 0011 as an ordinary integer; that route is NOT
    // allowed to reinterpret domain-qualified source as legacy numeric data.
    // Reuse the existing mixed typed-reader, not a new byte/semantic table.
    let linter_forms = parse_mixed_exact_domain(&linter_src)
        .expect("linter source preserves exact D1-D10 head widths");
    eval_parsed_expressions(&linter_forms, &mut session)
        .expect("load exact-domain linter");

    // Check if linter.my works by defining the threshold script
    let runner_src = r#"
        (def thresholds (quote ((max-size . 2000) (max-nesting . 30) (max-complexity . 24) (max-globals . 55) (max-effects . 20))))

        (def check-file-loop
          (lambda (path remaining all-violations)
            (cond
              ((атом? remaining) all-violations)
              ((атом? (quote fallback))
               ((lambda (violations)
                  (cond
                    ((атом? violations) (check-file-loop path (cdr remaining) all-violations))
                    ((атом? (quote fallback))
                     (check-file-loop path (cdr remaining) (cons (list path (car remaining) violations) all-violations)))))
                (lint-check (car remaining) thresholds))))))

        (def check-file
          (lambda (path)
            (check-file-loop path (read-all (read-file path)) (quote ()))))

        (def append-all
          (lambda (lists)
            (cond
              ((атом? lists) (quote ()))
              ((атом? (quote fallback))
               (append (car lists) (append-all (cdr lists)))))))

        (append-all
          (list
            (check-file "../../lib/core.lisp")
            (check-file "../../lib/linter.lisp")
            (check-file "../../lib/world.lisp")
            (check-file "../../lib/content-store.lisp")
            (check-file "../../lib/reason.lisp")))
    "#;

    match eval_program(runner_src, &mut session) {
        Ok(res) => {
            if !matches!(res.value, sens::Value::Nil) {
                panic!("Lint violations found:\n{}", res.value);
            }
        }
        Err(e) => {
            panic!("Error evaluating runner_src:\n{:?}", e);
        }
    }
}
