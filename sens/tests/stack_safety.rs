use sens::{eval_program, Exactness, Session, Value};
use std::rc::Rc;

fn build_long_list(count: usize) -> Value {
    let mut list = Value::Nil;
    for _ in 0..count {
        list = Value::Pair(Rc::new(Value::Nil), Rc::new(list));
    }
    list
}

#[test]
fn cons_chain_drop_does_not_overflow_stack() {
    let list = build_long_list(150_000);
    drop(list);
}

#[test]
fn cons_chain_clone_does_not_overflow_stack() {
    let list = build_long_list(150_000);
    // Value::clone() for Pair clones the Rcs, which is O(1) and stack-safe.
    // Dropping both lists relies on the iterative Drop mechanism.
    let _cloned = list.clone();
}

#[test]
fn shared_tails_do_not_overflow_stack() {
    let tail = build_long_list(150_000);
    let list1 = Value::Pair(
        Rc::new(Value::Number(1.0, Exactness::Exact)),
        Rc::new(tail.clone()),
    );
    let list2 = Value::Pair(Rc::new(Value::Number(2.0, Exactness::Exact)), Rc::new(tail));

    drop(list1); // Drops list1's head and its Rc to tail. tail's refcount goes from 2 to 1. No iterative drop for tail.
    drop(list2); // Drops list2's head and its Rc to tail. tail's refcount goes from 1 to 0. Iterative drop handles tail.
}

/// `length`/`map`/`filter`/`append` in lib/core.my build their result via a
/// tail-recursive `-onto` accumulator specifically so a deep list doesn't
/// grow the Rust call stack — this exercises that on a 100,000-element list
/// through the real evaluator (not the Rust-side Value construction the
/// tests above use), so a future non-tail-recursive rewrite of any of them
/// would fail here instead of only in production on a large enough list.
#[test]
fn core_lib_list_utilities_stay_stack_safe_on_a_long_list() {
    let mut session = Session::default();
    eval_program(include_str!("../../../lib/core.lisp"), &mut session).unwrap();
    let source = r#"
        (def build (lambda (n acc) (cond ((eq? n 0) acc) (t (build (- n 1) (cons n acc))))))
        (def big (build 100000 (quote ())))
        (length (map (lambda (x) (+ x 1)) (filter (lambda (x) (> x 50000)) (append big (quote ())))))
    "#;
    let result = eval_program(source, &mut session).unwrap();
    assert_eq!(result.value, Value::Number(50000.0, Exactness::Exact));
}

/// `scripts/symbol-table.my`'s own `sort-symbols`/`insert-sorted` (2026-08-10,
/// requested by the fpga-lisp session for a canonical symbol-name -> id
/// table) genuinely overflowed the Rust stack in a debug build at just 83
/// elements — the naive `(cons (car sorted) (insert-sorted sym (cdr
/// sorted)))` shape, not the tail-recursive one now in the file. Runs the
/// real script's own logic against lib/core.my (83 symbols as of this
/// writing) to guard against a future non-tail-recursive regression.
/// Mirrors scripts/symbol-table.lisp's own collect/sort functions rather
/// than running that file directly: the script's own tail calls
/// `(read-file "lib/core.lisp")` with a path relative to the repo root,
/// which isn't `cargo test`'s working directory (the crate root) --
/// `include_str!` sidesteps that entirely by embedding the text at
/// compile time. Keep this in sync with scripts/symbol-table.my if its
/// collect-symbols-onto/insert-sorted/sort-symbols logic changes.
#[test]
fn symbol_table_sort_stays_stack_safe() {
    let mut session = Session::default();
    eval_program(include_str!("../../../lib/core.lisp"), &mut session).unwrap();
    let helpers = r#"
        (def collect-symbols-onto
          (lambda (expr acc)
            (cond
              ((symbol? expr) (cond ((member? expr acc) acc) (t (cons expr acc))))
              ((atom? expr) () acc)
              ((atom? expr) (1) acc)
              (t (collect-symbols-onto (cdr expr) (collect-symbols-onto (car expr) acc))))))
        (def collect-all-symbols
          (lambda (forms acc)
            (cond
              ((atom? forms) () acc)
              ((atom? forms) (1) acc)
              (t (collect-all-symbols (cdr forms) (collect-symbols-onto (car forms) acc))))))
        (def insert-sorted-onto
          (lambda (sym before after)
            (cond
              ((atom? after) () (reverse-onto before (list sym)))
              ((atom? after) (1) (reverse-onto before (list sym)))
              ((string<? (symbol->string sym) (symbol->string (car after)))
               (reverse-onto before (cons sym after)))
              (t (insert-sorted-onto sym (cons (car after) before) (cdr after))))))
        (def insert-sorted (lambda (sym sorted) (insert-sorted-onto sym (quote ()) sorted)))
        (def sort-symbols-onto
          (lambda (remaining sorted)
            (cond
              ((atom? remaining) () sorted)
              ((atom? remaining) (1) sorted)
              (t (sort-symbols-onto (cdr remaining) (insert-sorted (car remaining) sorted))))))
        (def sort-symbols (lambda (symbols) (sort-symbols-onto symbols (quote ()))))
    "#;
    eval_program(helpers, &mut session).unwrap();

    let core_forms_source = format!(
        r#"(length (sort-symbols (collect-all-symbols (read-all {:?}) (quote ()))))"#,
        include_str!("../../../lib/core.lisp")
    );
    let result = eval_program(&core_forms_source, &mut session).unwrap();
    let Value::Number(count, _) = result.value else {
        panic!("length should return a number");
    };
    assert!(
        count >= 80.0,
        "expected at least ~80 symbols in lib/core.my, got {count}"
    );
}

#[test]
fn improper_lists_do_not_overflow_stack() {
    let count = 150_000;
    // Improper list ends in Number(42.0)
    let mut list = Value::Number(42.0, Exactness::Exact);
    for _ in 0..count {
        list = Value::Pair(Rc::new(Value::Nil), Rc::new(list));
    }
    drop(list);
}
