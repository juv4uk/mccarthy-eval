use sens::{eval_program, load_core_library, load_tcp_library, Session, Value};

#[test]
fn utf8_materialization_stays_stack_safe_on_a_worker_thread() {
    let handle = std::thread::Builder::new()
        .name("utf8-stack-witness".into())
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let mut session = Session::default();
            load_core_library(&mut session).unwrap();
            load_tcp_library(&mut session).unwrap();

            let bytes = std::iter::repeat_n("97", 1024)
                .collect::<Vec<_>>()
                .join(" ");
            let source = format!("(tcp-read-bytes->text (quote ({bytes})))");
            let result = eval_program(&source, &mut session)
                .expect("a 1 KiB ASCII payload should materialize without growing the Rust stack");

            let Value::String(ref text) = result.value else {
                panic!("valid ASCII bytes should decode to a string");
            };
            assert_eq!(text.len(), 1024);
            assert!(text.chars().all(|ch| ch == 'a'));
        })
        .expect("worker thread should start");

    handle.join().expect("UTF-8 worker must not overflow its stack");
}
