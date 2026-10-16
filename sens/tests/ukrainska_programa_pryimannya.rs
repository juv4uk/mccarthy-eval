use sens::{eval_program, Session};

const УКРАЇНСЬКА_ПРОГРАМА: &str =
    include_str!("../../../tests/fixtures/rivnopravnist-uk.lisp");

// україномовна_програма_приймання_не_потребує_латинської_розкладки was a
// keyboard/text-policy lint, relocated to `cargo xtask verify` per
// TEST-ARCHITECTURE-1 step 4 -- see
// crates/xtask/src/checks.rs::ukrainska_prohrama_pryinnyattia_ne_potrebuie_latynskoi_rozkladky.

#[test]
fn україномовна_програма_приймання_виконується_самостійно() {
    let mut сесія = Session::default();
    let результат = eval_program(УКРАЇНСЬКА_ПРОГРАМА, &mut сесія)
        .expect("україномовна acceptance-програма повинна виконуватися")
        .value
        .to_string();

    assert_eq!(
        результат, "гаразд",
        "українська поверхня повинна самостійно пройти acceptance-програму"
    );
}
