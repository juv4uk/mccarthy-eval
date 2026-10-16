use std::collections::BTreeSet;

use sens::{
    eval_program, load_core_library, parse,
    semantic_registry_export::semantic_id_for_admitted_surface,
    syntax::{Expr, ExprKind},
    Session,
};

const INVENTORY: &str = include_str!("../../../tests/data/macro-generated-heads-1485.tsv");
const CORE: &str = include_str!("../../../lib/core.lisp");
const CORE4: &str = include_str!("../../../lib/core4.lisp");

#[derive(Debug, Clone, Eq, Ord, PartialEq, PartialOrd)]
struct Row {
    path: String,
    surface: String,
    exact_sens: String,
    class: String,
}

fn inventory_rows() -> Vec<Row> {
    INVENTORY
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .map(|line| {
            let parts: Vec<_> = line.split('\t').collect();
            assert_eq!(parts.len(), 4, "bad #1485 inventory row: {line}");
            Row {
                path: parts[0].to_string(),
                surface: parts[1].to_string(),
                exact_sens: parts[2].to_string(),
                class: parts[3].to_string(),
            }
        })
        .collect()
}

fn sens_for_admitted_surface(surface: &str) -> Option<sens::Sens8> {
    semantic_id_for_admitted_surface(surface)
}

fn is_quote_head(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::Sid(sens) => *sens == sens::sens!(00000001),
        ExprKind::Symbol(surface) => {
            sens_for_admitted_surface(surface.as_ref()) == Some(sens::sens!(00000001))
        }
        _ => false,
    }
}

fn collect_quoted_admitted_single_atoms(
    path: &str,
    expr: &Expr,
    out: &mut BTreeSet<Row>,
) {
    match &expr.kind {
        ExprKind::List(items) => {
            if items.len() == 2 && is_quote_head(&items[0]) {
                if let ExprKind::Symbol(surface) = &items[1].kind {
                    if let Some(exact_sens) = sens_for_admitted_surface(surface.as_ref()) {
                        out.insert(Row {
                            path: path.to_string(),
                            surface: surface.to_string(),
                            exact_sens: exact_sens.to_string(),
                            class: String::new(),
                        });
                    }
                }

                // The quote payload is data. Do not reinterpret nested lists inside it
                // as executable quote forms.
                return;
            }

            for item in items.iter() {
                collect_quoted_admitted_single_atoms(path, item, out);
            }
        }
        ExprKind::Pair(head, tail) => {
            collect_quoted_admitted_single_atoms(path, head, out);
            collect_quoted_admitted_single_atoms(path, tail, out);
        }
        _ => {}
    }
}

fn quoted_admitted_single_atoms(path: &str, source: &str) -> BTreeSet<Row> {
    let expressions =
        parse(source).unwrap_or_else(|error| panic!("{path}: canonical parser failed: {error:?}"));
    let mut out = BTreeSet::new();

    for expr in &expressions {
        collect_quoted_admitted_single_atoms(path, expr, &mut out);
    }

    out
}

fn discovered_rows_without_class() -> BTreeSet<Row> {
    let mut rows = quoted_admitted_single_atoms("lib/core.lisp", CORE);
    rows.extend(quoted_admitted_single_atoms("lib/core4.lisp", CORE4));
    rows
}

#[test]
fn discovery_is_structural_and_recognizes_quote_surface_aliases() {
    let rows = quoted_admitted_single_atoms(
        "synthetic",
        r#"
          ; (00000001 or)
          "(00000001 let)"
          (quote and)
          (00000001 binary)
          (00000001 (quote let*))
        "#,
    );

    let expected = BTreeSet::from([
        Row {
            path: "synthetic".to_string(),
            surface: "and".to_string(),
            exact_sens: "10011010".to_string(),
            class: String::new(),
        },
        Row {
            path: "synthetic".to_string(),
            surface: "binary".to_string(),
            exact_sens: "10101001".to_string(),
            class: String::new(),
        },
    ]);

    assert_eq!(
        rows, expected,
        "canonical parsing must ignore comments/strings, admit quote surfaces, and stop at quoted data"
    );
}

#[test]
fn inventory_covers_every_quoted_admitted_single_atom_in_core_and_core4() {
    let inventoried: BTreeSet<_> = inventory_rows()
        .into_iter()
        .map(|mut row| {
            row.class.clear();
            row
        })
        .collect();

    assert_eq!(
        discovered_rows_without_class(),
        inventoried,
        "#1485 inventory must change whenever quoted admitted single-atom values change"
    );
}

#[test]
fn inventory_has_only_explicit_data_or_code_template_classes() {
    let rows = inventory_rows();
    assert_eq!(
        rows.len(),
        16,
        "current audit is exactly eight rows per core profile"
    );

    for row in &rows {
        assert!(
            matches!(
                row.class.as_str(),
                "ordinary-data" | "code-template-operator"
            ),
            "unclassified #1485 row: {row:?}"
        );
    }

    let ordinary: Vec<_> = rows
        .iter()
        .filter(|row| row.class == "ordinary-data")
        .collect();
    assert_eq!(ordinary.len(), 2);
    assert!(ordinary.iter().all(|row| row.surface == "binary"));
    assert!(ordinary.iter().all(|row| row.exact_sens == "10101001"));
}

#[test]
fn ordinary_data_is_not_promoted_to_exact_sens() {
    for (path, source) in [("lib/core.lisp", CORE), ("lib/core4.lisp", CORE4)] {
        assert!(
            source.contains("(00000001 binary)"),
            "{path}: the admitted-looking symbol binary is ordinary quoted data and must remain data"
        );
        assert!(
            !source.contains("(00000001 10101001)"),
            "{path}: #1485 must not blanket-rewrite ordinary quoted data to exact SENS"
        );
    }
}

#[test]
fn code_template_operator_rows_name_the_exact_registry_function() {
    for row in inventory_rows()
        .into_iter()
        .filter(|row| row.class == "code-template-operator")
    {
        assert_eq!(
            sens_for_admitted_surface(&row.surface).map(|sens| sens.to_string()),
            Some(row.exact_sens.clone()),
            "inventory exact SENS must be derived from the current registry: {row:?}"
        );
    }
}

#[test]
#[ignore = "RED witness for #1485: unignore when production macro/code templates emit exact SENS"]
fn code_template_operators_never_reintroduce_surface_heads() {
    let discovered = discovered_rows_without_class();
    let offenders: Vec<_> = inventory_rows()
        .into_iter()
        .filter(|row| row.class == "code-template-operator")
        .filter(|row| {
            let mut structural = row.clone();
            structural.class.clear();
            discovered.contains(&structural)
        })
        .collect();

    assert!(
        offenders.is_empty(),
        "#1485 RED: code-producing templates still quote human surface heads: {offenders:#?}"
    );
}


#[test]
fn ordinary_admitted_binary_symbol_remains_user_data() {
    let mut session = Session::default();
    load_core_library(&mut session).expect("canonical core bootstrap must load");

    let value = eval_program("(binary 8)", &mut session)
        .expect("binary descriptor must remain ordinary language data")
        .value
        .to_string();

    assert_eq!(
        value, "(binary 8)",
        "#1485 negative control: admitted surface-looking data must not be promoted to exact SENS"
    );
}
