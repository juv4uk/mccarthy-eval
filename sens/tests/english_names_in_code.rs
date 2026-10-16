//! Де в коді ще стоять англійські імена функцій з таблиці функцій.
//!
//! Джерело імен — простір `en` таблиці функцій
//! (`lib/surface/semantic-registry.lisp` через
//! `semantic_registry_generated.rs`). Код = коди СЕНС, імена — лише
//! поверхня REPL, тож кожне англійське ім'я в коді — місце, яке ще не
//! переведене на СЕНС.
//!
//! Три види місць:
//!
//! - `lisp` — токен у коді мови (`.lisp`, `.sens`, ...): голова виклику
//!   (`(car x)`), визначення (`(00001001 reverse ...)`) чи посилання на
//!   функцію (`(map car xs)`). Рядки й коментарі пропускаються.
//! - `lisp-дані` — те саме ім'я під `'`, `` ` ``, `quote` чи `00000001`:
//!   дані, а не код (окремо, щоб не заступати код).
//! - `rust` — рядковий літерал Rust, який дорівнює імені (`"car"`): Rust
//!   знає функцію за іменем, а не за кодом.
//! - `rust-lisp` / `rust-lisp-дані` — код мови (чи його дані) всередині
//!   рядкового літерала Rust (`eval("(car x)")`).
//!
//! Файли даних, які читаються через `read-all` без `quote`, сканер не
//! відрізняє від коду — вони йдуть як `lisp`.
//!
//! Сама таблиця функцій і згенеровані з неї проєкції (`*_generated.rs`,
//! `lib/generated/`) — не код, а джерело імен; вони не скануються.
//!
//! Храповик: `tests/data/english-names-baseline.tsv` (вид, файл, ім'я,
//! кількість). Тест падає, якщо місць стало більше, і показує рядки.
//! `SENS_BASELINE_UPDATE=1` переписує базу (лише свідомо).
//! Повний перелік місць (файл:рядок):
//! `cargo test -p sens --test english_names_in_code -- --ignored`.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
}

/// Англійські імена з таблиці функцій.
fn english_names() -> BTreeSet<String> {
    let path = repo_root().join("crates/sens/src/semantic_registry_generated.rs");
    let text = fs::read_to_string(path).unwrap();
    let names: BTreeSet<String> = text
        .split("SemanticSurface { namespace: \"en\", name: \"")
        .skip(1)
        .map(|rest| rest.chars().take_while(|c| *c != '"').collect::<String>())
        .filter(|name| !name.is_empty())
        .collect();
    assert!(names.len() > 100, "англійські імена не прочитано ({})", names.len());
    names
}

/// Історичні вимірювання та старі еталонні бенчмарки, а не діючі джерела.
/// Перелік навмисно замкнутий: новий executable-код під benchmarks/ мусить
/// проходити ті самі перевірки, що й увесь інший код СЕНС.
fn is_historical_benchmark_data(rel: &str) -> bool {
    const LEGACY_FIXTURES: &[&str] = &[
        "benchmarks/arithmetic.lisp",
        "benchmarks/closures.lisp",
        "benchmarks/lists.lisp",
        "benchmarks/parser.lisp",
        "benchmarks/recursion.lisp",
        "benchmarks/core-coremath-selector-convergence/src/main.rs",
        "benchmarks/core-math-binary-exec/src/lib.rs",
        "benchmarks/core-math-binary-exec/src/main.rs",
        "benchmarks/core-math-binary-growth/src/lib.rs",
        "benchmarks/core-math-binary-growth/src/main.rs",
        "benchmarks/core-math-binary-seeds/src/lib.rs",
        "benchmarks/core-math-binary-seeds/src/main.rs",
        "benchmarks/foundation-ladder/foundation_ladder.rs",
        "benchmarks/self-description/bench.rs",
        "benchmarks/semantic-tree/bench.rs",
        "benchmarks/sens-surface/results/20260927-three-way/phase_bench_legacy.rs",
    ];
    const PINNED_RESULT_PREFIXES: &[&str] = &[
        "benchmarks/sens-surface/results/20260925-211421-33bfb53a/programs/",
        "benchmarks/sens-surface/results/20260925-213342-33bfb53a/programs/",
        "benchmarks/sens-surface/results/20260925-225540-b49a87ea/programs/",
        "benchmarks/sens-surface/results/20260925-icount-33bfb53a/programs/",
        "benchmarks/sens-surface/results/20260925-inprocess-33bfb53a/programs/",
    ];
    LEGACY_FIXTURES.contains(&rel)
        || (rel.ends_with(".lisp")
            && PINNED_RESULT_PREFIXES.iter().any(|prefix| rel.starts_with(prefix)))
}

/// Таблиця функцій і її проєкції — джерело імен, не код.
fn is_table_source(rel: &str) -> bool {
    rel == "lib/surface/semantic-registry.lisp"
        || rel == "lib/surface/function-signatures.lisp"
        // Canonical D1+ human-readable domain tables are surface/registry data,
        // not executable language code.
        || rel.starts_with("lib/domains/")
        // Generated machine evidence tables are data projections. Keep exact
        // paths here so generated executable Lisp is not silently exempted.
        || rel == "lib/machine/encoding/admitted-iclass-index.lisp"
        || rel == "lib/machine/encoding/coverage.lisp"
        // Exact data-only contract/provenance tables currently visible to the
        // scanner.  Keep this path-specific: contracts as a directory are not
        // exempt from executable-name migration.
        || rel == "contracts/compiler-gpu-execution-packet-v1.lisp"
        || rel == "contracts/d8-ratification.lisp"
        || rel == "contracts/d9-ratification.lisp"
        || rel.starts_with("lib/generated/")
        || rel.ends_with("_generated.rs")
        || rel.starts_with("crates/sens/tests/data/")
        // Pinned historical benchmark evidence only; never blanket-exempt a tree.
        || is_historical_benchmark_data(rel)
        // Сам цей тест: імена в його перевірках сканера — вхідні дані.
        || rel == "crates/sens/tests/english_names_in_code.rs"
}

fn explicit_nonsemantic_lisp_evidence(text: &str) -> bool {
    let header = text.lines().take(24).collect::<Vec<_>>().join("\n").to_lowercase();
    header.contains("(role research-only)")
        || header.contains("(semantic-authority-change none)")
        || header.contains("inventory only")
        || header.contains("не language-contract")
}

fn rust_test_is_semantic_authority(rel: &str) -> bool {
    if !rel.starts_with("crates/") || !rel.contains("/tests/") || !rel.ends_with(".rs") {
        return false;
    }
    let inventory = fs::read_to_string(repo_root().join("tests/authority-inventory.tsv"))
        .unwrap_or_default();
    inventory.lines().any(|line| {
        let mut fields = line.split('\t');
        let Some(path) = fields.next() else { return false };
        let _site = fields.next();
        let class = fields.next().unwrap_or_default();
        let normalized = path.replace("crates/my-lisp/", "crates/sens/");
        normalized == rel && class == "semantic-authority"
    })
}

/// This specific Lisp-shaped document declares itself operational doctrine,
/// not an executable SENS source and not language-contract authority.
/// Do NOT exempt a whole directory or unmarked Lisp files: the English
/// name migration ratchet must still catch actual executable growth.
fn is_operational_doctrine_data(rel: &str, text: &str) -> bool {
    rel == "knowledge/sens-primary.lisp"
        && text.lines().take(8).any(|line| {
            line.trim() == "; Status: operational doctrine (not language-contract authority)."
        })
        && text.lines().take(20).any(|line| line.trim() == "(sens-primary/2")
}

fn classified_kind(rel: &str, text: &str, base_kind: &'static str) -> &'static str {
    if rel.starts_with("crates/")
        && rel.contains("/tests/")
        && rel.ends_with(".rs")
        && !rust_test_is_semantic_authority(rel)
    {
        return "rust-test-instrument";
    }
    if !rel.ends_with(".rs")
        && (explicit_nonsemantic_lisp_evidence(text) || is_operational_doctrine_data(rel, text))
    {
        return "lisp-evidence";
    }
    base_kind
}

fn ratchet_enforced_kind(kind: &str) -> bool {
    !matches!(
        kind,
        "rust-test-instrument" | "lisp-evidence" | "rust-contract-data" | "rust-evidence-data" | "rust-cli-surface"
    )
}

fn rust_nonsemantic_data_kind(
    rel: &str,
    line_text: &str,
    literal: &str,
) -> Option<&'static str> {
    let source = literal.trim_start();

    if rel == "crates/xtask/src/compiler_export.rs"
        && source.starts_with("(compiler-semantic-request")
    {
        return Some("rust-contract-data");
    }

    if rel == "crates/sens/src/gpu_oracle.rs"
        && literal == "numeric-buffer-map"
        && line_text.contains("forbidden_legacy_operation: \"numeric-buffer-map\".to_string()")
    {
        return Some("rust-evidence-data");
    }

    // The command line dispatch of sens-trit is a human-facing surface, not
    // an English-named SENS semantic primitive. Exempt EXACTLY this single
    // match arm; an ordinary Rust `"eval"` elsewhere remains ratchet debt.
    if rel == "crates/sens-cli/src/bin/sens-trit.rs"
        && literal == "eval"
        && line_text.trim() == "\"eval\" => {"
    {
        return Some("rust-cli-surface");
    }

    // This exact Lisp form is a negative test fixture embedded in a
    // production module's #[cfg(test)] section. It intentionally checks that
    // English "car" does NOT mint a current identity; class it as test input,
    // while equivalent strings elsewhere remain executable-name debt.
    if rel == "crates/sens/src/mixed_source.rs"
        && literal == "(car x)"
        && line_text.trim().starts_with("for source in [\"(car x)\"")
    {
        return Some("rust-test-instrument");
    }

    None
}

fn files() -> Vec<(PathBuf, String)> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<(PathBuf, String)>) {
        let Ok(entries) = fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if path.is_dir() {
                if name == "vendor" || name == ".git" || name == "node_modules" || name.starts_with("target") {
                    continue;
                }
                walk(root, &path, out);
            } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if ["lisp", "sens", "my", "wsm", "всм", "мій", "лісп", "rs"].contains(&ext) {
                    let rel = path.strip_prefix(root).unwrap_or(&path).display().to_string();
                    if !is_table_source(&rel) {
                        out.push((path, rel));
                    }
                }
            }
        }
    }
    let root = repo_root();
    let mut out = Vec::new();
    walk(&root, &root, &mut out);
    out.sort();
    out
}

/// Токени коду мови: (рядок, токен, у-даних). Рядки й коментарі
/// пропускаються. «У даних» — усередині `'(...)`, `` `(...) `` чи
/// `(quote ...)` / `(00000001 ...)`: там ім'я є даними, а не кодом.
fn lisp_tokens(text: &str, first_line: usize) -> Vec<(usize, String, bool)> {
    let mut out = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let (mut i, mut line) = (0, first_line);
    // Для кожної відкритої дужки: чи це дані, і чи вже прочитано голову.
    let mut stack: Vec<(bool, bool)> = Vec::new();
    let mut quote_prefix = false;
    while i < chars.len() {
        let c = chars[i];
        if c == '\n' {
            line += 1;
            i += 1;
        } else if c == ';' {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        } else if c == '"' {
            i += 1;
            while i < chars.len() && chars[i] != '"' {
                if chars[i] == '\\' {
                    i += 1;
                } else if chars[i] == '\n' {
                    line += 1;
                }
                i += 1;
            }
            i += 1;
            quote_prefix = false;
        } else if c == '\'' || c == '`' {
            quote_prefix = true;
            i += 1;
        } else if c == '(' || c == '[' {
            let data = stack.last().is_some_and(|(d, _)| *d) || quote_prefix;
            stack.push((data, false));
            quote_prefix = false;
            i += 1;
        } else if c == ')' || c == ']' {
            stack.pop();
            i += 1;
        } else if c.is_whitespace() || c == ',' || c == '@' {
            i += 1;
        } else {
            let start = i;
            while i < chars.len() && !chars[i].is_whitespace() && !"()[]\"';`,".contains(chars[i]) {
                i += 1;
            }
            let token: String = chars[start..i].iter().collect();
            let data = quote_prefix || stack.last().is_some_and(|(d, _)| *d);
            if let Some(top) = stack.last_mut() {
                if !top.1 {
                    top.1 = true;
                    if !top.0 && (token == "quote" || token == "00000001") {
                        top.0 = true;
                    }
                }
            }
            out.push((line, token, data));
            quote_prefix = false;
        }
    }
    out
}

/// Рядкові літерали Rust: (рядок початку, вміст). Коментарі й символьні
/// літерали пропускаються; сирі рядки `r#"..."#` підтримуються.
fn rust_strings(text: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let (mut i, mut line) = (0, 1);
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        if c == '\n' {
            line += 1;
            i += 1;
        } else if c == '/' && next == Some('/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        } else if c == '/' && next == Some('*') {
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                if chars[i] == '\n' {
                    line += 1;
                }
                i += 1;
            }
            i += 2;
        } else if c == '\'' {
            // Символьний літерал ('x', '\n', '"') або час життя ('a).
            if next == Some('\\') {
                i += 2;
                while i < chars.len() && chars[i] != '\'' {
                    i += 1;
                }
                i += 1;
            } else if chars.get(i + 2) == Some(&'\'') {
                i += 3;
            } else {
                i += 1;
            }
        } else if c == 'r' && (next == Some('"') || next == Some('#'))
            && (i == 0 || !(chars[i - 1].is_alphanumeric() || chars[i - 1] == '_'))
        {
            let mut j = i + 1;
            let mut hashes = 0;
            while j < chars.len() && chars[j] == '#' {
                hashes += 1;
                j += 1;
            }
            if chars.get(j) != Some(&'"') {
                i += 1;
                continue;
            }
            let start_line = line;
            j += 1;
            let start = j;
            loop {
                if j >= chars.len() {
                    break;
                }
                if chars[j] == '"' && (0..hashes).all(|k| chars.get(j + 1 + k) == Some(&'#')) {
                    break;
                }
                if chars[j] == '\n' {
                    line += 1;
                }
                j += 1;
            }
            out.push((start_line, chars[start..j.min(chars.len())].iter().collect()));
            i = j + 1 + hashes;
        } else if c == '"' {
            let start_line = line;
            let mut value = String::new();
            i += 1;
            while i < chars.len() && chars[i] != '"' {
                if chars[i] == '\\' {
                    match chars.get(i + 1) {
                        Some('n') => value.push('\n'),
                        Some('\n') => line += 1,
                        Some(other) => value.push(*other),
                        None => {}
                    }
                    i += 2;
                    continue;
                }
                if chars[i] == '\n' {
                    line += 1;
                }
                value.push(chars[i]);
                i += 1;
            }
            i += 1;
            out.push((start_line, value));
        } else {
            i += 1;
        }
    }
    out
}

fn rust_literal_has_lisp_source(literal: &str) -> bool {
    let source = literal.trim_start();
    source.starts_with('(')
}
/// Одне місце: (вид, файл, рядок, ім'я).
type Place = (&'static str, String, usize, String);

fn places() -> Vec<Place> {
    let names = english_names();
    let mut out = Vec::new();
    for (path, rel) in files() {
        let Ok(text) = fs::read_to_string(&path) else { continue };
        if rel.ends_with(".rs") {
            for (line, literal) in rust_strings(&text) {
                let line_text = text.lines().nth(line.saturating_sub(1)).unwrap_or("");
                let data_kind = rust_nonsemantic_data_kind(&rel, line_text, &literal);
                if names.contains(literal.as_str()) {
                    let kind =
                        data_kind.unwrap_or_else(|| classified_kind(&rel, &text, "rust"));
                    out.push((kind, rel.clone(), line, literal));
                } else if rust_literal_has_lisp_source(&literal) {
                    for (l, token, data) in lisp_tokens(&literal, line) {
                        if names.contains(&token) {
                            let base_kind = if data { "rust-lisp-дані" } else { "rust-lisp" };
                            let kind =
                                data_kind.unwrap_or_else(|| classified_kind(&rel, &text, base_kind));
                            out.push((kind, rel.clone(), l, token));
                        }
                    }
                }
            }
        } else {
            for (line, token, data) in lisp_tokens(&text, 1) {
                if names.contains(&token) {
                    let base_kind = if data { "lisp-дані" } else { "lisp" };
                    let kind = classified_kind(&rel, &text, base_kind);
                    out.push((kind, rel.clone(), line, token));
                }
            }
        }
    }
    out
}

fn counts(places: &[Place]) -> BTreeMap<String, usize> {
    let mut out = BTreeMap::new();
    for (kind, file, _, name) in places {
        if !ratchet_enforced_kind(kind) {
            continue;
        }
        *out.entry(format!("{kind}\t{file}\t{name}")).or_insert(0) += 1;
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DebtMetrics {
    sites: usize,
    files: usize,
}

fn debt_metrics(places: &[Place]) -> DebtMetrics {
    let enforced: Vec<&Place> = places
        .iter()
        .filter(|(kind, _, _, _)| ratchet_enforced_kind(kind))
        .collect();
    let files: BTreeSet<&str> = enforced.iter().map(|(_, file, _, _)| file.as_str()).collect();
    DebtMetrics {
        sites: enforced.len(),
        files: files.len(),
    }
}

fn baseline_entry_enforced(key: &str) -> bool {
    let mut fields = key.splitn(3, '\t');
    let Some(kind) = fields.next() else { return false };
    let Some(file) = fields.next() else { return false };
    let _name = fields.next();

    if !ratchet_enforced_kind(kind) || is_table_source(file) {
        return false;
    }
    if file.starts_with("crates/")
        && file.contains("/tests/")
        && file.ends_with(".rs")
        && !rust_test_is_semantic_authority(file)
    {
        return false;
    }
    if !file.ends_with(".rs") {
        let text = fs::read_to_string(repo_root().join(file)).unwrap_or_default();
        if explicit_nonsemantic_lisp_evidence(&text) || is_operational_doctrine_data(file, &text) {
            return false;
        }
    }
    true
}

fn baseline_debt_metrics(baseline: &BTreeMap<String, usize>) -> DebtMetrics {
    let mut sites = 0usize;
    let mut files = BTreeSet::new();
    for (key, count) in baseline {
        if !baseline_entry_enforced(key) {
            continue;
        }
        sites += *count;
        if let Some((_, rest)) = key.split_once('\t') {
            if let Some((file, _)) = rest.split_once('\t') {
                files.insert(file);
            }
        }
    }
    DebtMetrics {
        sites,
        files: files.len(),
    }
}

fn emit_debt_metric(
    current: DebtMetrics,
    baseline: DebtMetrics,
    growth_sites: usize,
    growth_files: usize,
    growth_keys: usize,
) {
    let delta_sites = current.sites as isize - baseline.sites as isize;
    let delta_files = current.files as isize - baseline.files as isize;
    let line = format!(
        "semantic-migration-debt-v1 remaining_sites={} remaining_files={} baseline_sites={} baseline_files={} delta_sites={:+} delta_files={:+} growth_sites={} growth_files={} growth_keys={}",
        current.sites,
        current.files,
        baseline.sites,
        baseline.files,
        delta_sites,
        delta_files,
        growth_sites,
        growth_files,
        growth_keys,
    );
    eprintln!("{line}");

    if let Ok(path) = std::env::var("GITHUB_STEP_SUMMARY") {
        if let Ok(mut out) = fs::OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(
                out,
                "### Semantic migration debt\n\n- remaining executable-English sites: **{}**\n- production files: **{}**\n- classifier-normalized baseline: **{} sites / {} files**\n- delta: **{:+} sites / {:+} files**\n- ratchet growth: **{} sites / {} files / {} keys**\n",
                current.sites,
                current.files,
                baseline.sites,
                baseline.files,
                delta_sites,
                delta_files,
                growth_sites,
                growth_files,
                growth_keys,
            );
        }
    }
}

fn baseline_path() -> PathBuf {
    repo_root().join("crates/sens/tests/data/english-names-baseline.tsv")
}

fn read_baseline() -> BTreeMap<String, usize> {
    let text = fs::read_to_string(baseline_path()).unwrap_or_default();
    text.lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| {
            let (key, count) = l.rsplit_once('\t')?;
            Some((key.to_owned(), count.parse().ok()?))
        })
        .collect()
}

fn summary(places: &[Place]) -> String {
    let mut by_kind: BTreeMap<&str, usize> = BTreeMap::new();
    let mut by_name: BTreeMap<&str, usize> = BTreeMap::new();
    let files: BTreeSet<&str> = places.iter().map(|(_, f, _, _)| f.as_str()).collect();
    for (kind, _, _, name) in places {
        *by_kind.entry(kind).or_insert(0) += 1;
        *by_name.entry(name).or_insert(0) += 1;
    }
    let mut top: Vec<_> = by_name.into_iter().collect();
    top.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    let top: Vec<String> = top.iter().take(15).map(|(n, c)| format!("{n} {c}")).collect();
    format!(
        "англійських імен у коді: {} місць у {} файлах; за видом: {:?}; найчастіші: {}",
        places.len(),
        files.len(),
        by_kind,
        top.join(", ")
    )
}

#[test]
fn human_cli_eval_dispatch_cannot_mint_an_english_function_exemption() {
    let relative = "crates/sens-cli/src/bin/sens-trit.rs";
    let source = fs::read_to_string(repo_root().join(relative))
        .expect("read exact real CLI");
    let arms: Vec<_> = source.lines()
        .filter(|line| line.trim() == "\"eval\" => {")
        .collect();
    assert_eq!(arms.len(), 1, "the reviewed CLI dispatch shape must not drift");
    assert!(english_names().contains("eval"), "keep the real semantic name scanned");
    assert_eq!(
        rust_nonsemantic_data_kind(relative, arms[0], "eval"),
        Some("rust-cli-surface"),
    );
    assert!(!ratchet_enforced_kind("rust-cli-surface"));
    for (path, line) in [
        (relative, "let function_name = \"eval\";"),
        (relative, "\"eval\" => execute_host_command(),"),
        ("crates/sens/src/eval/mod.rs", "\"eval\" => {"),
        ("crates/sens-cli/src/bin/other.rs", "\"eval\" => {"),
    ] {
        assert_eq!(
            rust_nonsemantic_data_kind(path, line, "eval"),
            None,
            "English executable names must remain debt outside the exact reviewed CLI arm",
        );
    }
    assert!(ratchet_enforced_kind("rust"));
}

#[test]
fn embedded_mixed_source_negative_fixture_is_test_input_only() {
    let fixture_line =
        r#"for source in ["(car x)", "(CONS x y)", "(00000101 x)", "(невідоме x)"] {"#;
    assert_eq!(
        rust_nonsemantic_data_kind(
            "crates/sens/src/mixed_source.rs",
            fixture_line,
            "(car x)",
        ),
        Some("rust-test-instrument"),
    );

    // No broad path/name exemption: production uses of the same source text
    // still contribute to the English-name ratchet.
    for line in [
        r#"let source = "(car x)";"#,
        r#"parse_mixed_exact_domain("(car x)")"#,
    ] {
        assert_eq!(
            rust_nonsemantic_data_kind(
                "crates/sens/src/mixed_source.rs",
                line,
                "(car x)",
            ),
            None,
        );
    }
}

#[test]
fn english_names_in_code_never_grow() {
    let places = places();
    let current = counts(&places);
    if std::env::var("SENS_BASELINE_UPDATE").as_deref() == Ok("1") {
        let mut text = String::from(
            "# Англійські імена з таблиці функцій у коді — храповик (див. tests/english_names_in_code.rs)\n\
             # вид\tфайл\tім'я\tкількість\n",
        );
        for (key, count) in &current {
            text.push_str(&format!("{key}\t{count}\n"));
        }
        fs::create_dir_all(baseline_path().parent().unwrap()).unwrap();
        fs::write(baseline_path(), text).unwrap();
        return;
    }
    let baseline = read_baseline();
    assert!(!baseline.is_empty(), "бази немає: SENS_BASELINE_UPDATE=1 cargo test ...");
    let current_debt = debt_metrics(&places);
    let baseline_debt = baseline_debt_metrics(&baseline);
    let mut grown = Vec::new();
    let mut growth_sites = 0usize;
    let mut growth_files = BTreeSet::new();
    let mut growth_keys = 0usize;
    for (key, count) in &current {
        let base = baseline.get(key).copied().unwrap_or(0);
        if *count > base {
            let mut parts = key.splitn(3, '\t');
            let (kind, file, name) = (parts.next().unwrap(), parts.next().unwrap(), parts.next().unwrap());
            let lines: Vec<String> = places
                .iter()
                .filter(|(k, f, _, n)| *k == kind && f == file && n == name)
                .map(|(_, _, l, _)| l.to_string())
                .collect();
            growth_sites += *count - base;
            growth_files.insert(file.to_owned());
            growth_keys += 1;
            grown.push(format!("{file}:{} {name} ({kind}): {base} -> {count}", lines.join(",")));
        }
    }
    emit_debt_metric(
        current_debt,
        baseline_debt,
        growth_sites,
        growth_files.len(),
        growth_keys,
    );
    eprintln!("{}", summary(&places));
    assert!(
        grown.is_empty(),
        "англійських імен у коді стало більше — пишіть кодами СЕНС:\n{}",
        grown.join("\n")
    );
}

#[test]
#[ignore = "мета: у коді немає англійських імен — лише коди СЕНС"]
fn no_english_names_in_code() {
    let places = places();
    let listing: Vec<String> = places
        .iter()
        .map(|(kind, file, line, name)| format!("{file}:{line}\t{name}\t{kind}"))
        .collect();
    assert!(places.is_empty(), "{}\n{}", summary(&places), listing.join("\n"));
}

#[test]
fn sens_primary_operational_doctrine_does_not_count_as_executable_english() {
    let rel = "knowledge/sens-primary.lisp";
    let content = fs::read_to_string(repo_root().join(rel)).unwrap();
    assert!(is_operational_doctrine_data(rel, &content));
    assert_eq!(classified_kind(rel, &content, "lisp"), "lisp-evidence");
    assert!(!ratchet_enforced_kind(classified_kind(rel, &content, "lisp")));

    // A different executable source with identical Lisp-shaped content is
    // never automatically exempted; nor is a source lacking the header.
    assert!(!is_operational_doctrine_data("lib/core.lisp", &content));
    assert!(ratchet_enforced_kind(classified_kind("lib/core.lisp", &content, "lisp")));
    assert!(ratchet_enforced_kind(classified_kind(rel, "(identity 1)", "lisp")));
}

#[test]
fn scanners_find_names_in_lisp_and_rust() {
    let lisp = lisp_tokens("(map car xs) ; cdr\n\"cons\" '(length 1) (quote (list)) (00000001 reverse) (f x)", 1);
    let tokens: Vec<(&str, bool)> = lisp.iter().map(|(_, t, d)| (t.as_str(), *d)).collect();
    assert_eq!(
        tokens,
        [
            ("map", false), ("car", false), ("xs", false),
            ("length", true), ("1", true),
            ("quote", false), ("list", true),
            ("00000001", false), ("reverse", true),
            ("f", false), ("x", false),
        ]
    );
    assert_eq!(lisp[3].0, 2);

    assert_eq!(
        classified_kind("crates/sens/tests/numeric_wire.rs", "", "rust-lisp"),
        "rust-test-instrument"
    );
    assert_eq!(
        classified_kind("crates/sens-host/tests/native_lisp_bytes.rs", "", "rust-lisp"),
        "rust-test-instrument",
        "tests in other crates are instruments unless explicitly semantic-authority"
    );
    assert!(is_table_source("lib/domains/d9.lisp"));
    assert!(is_table_source("lib/machine/encoding/admitted-iclass-index.lisp"));
    assert!(is_table_source("lib/machine/encoding/coverage.lisp"));
    assert!(is_table_source("contracts/compiler-gpu-execution-packet-v1.lisp"));
    assert!(is_table_source("contracts/d8-ratification.lisp"));
    assert!(is_table_source("contracts/d9-ratification.lisp"));
    assert!(!is_table_source("contracts/core-universal-contract.lisp"));
    assert!(!is_table_source("lib/compiler-nucleus.lisp"));
    assert!(!is_table_source("lib/machine/encoding/x86-64.lisp"));
    // A new benchmark, archive or old xtask file must NOT inherit a
    // blanket English-identity exemption from its parent directory.
    assert!(is_table_source("benchmarks/arithmetic.lisp"));
    assert!(is_table_source(
        "benchmarks/sens-surface/results/20260925-211421-33bfb53a/programs/closures-en.lisp"
    ));
    assert!(!is_table_source("benchmarks/new-executable.lisp"));
    assert!(!is_table_source("benchmarks/new-module/src/main.rs"));
    assert!(!is_table_source("benchmarks/sens-surface/results/20261010-new/programs/fib-en.lisp"));
    assert!(!is_table_source("archive/new-executable.lisp"));
    assert!(!is_table_source("crates/xtask.bak/src/main.rs"));
    assert!(!rust_literal_has_lisp_source("CDR: list accessor (structural)"));
    assert!(!rust_literal_has_lisp_source("CONS: pair construction (structural)"));
    assert!(rust_literal_has_lisp_source("(атом? x)"));
    assert!(rust_literal_has_lisp_source("  (як-є radio)"));
    assert_eq!(
        classified_kind("crates/sens/tests/mccarthy.rs", "", "rust-lisp"),
        "rust-lisp",
        "explicit semantic-authority test must stay inside the executable-name ratchet"
    );
    assert!(explicit_nonsemantic_lisp_evidence(
        "; Inventory only: mechanism evidence, no semantic authority\n(foo car)"
    ));
    assert_eq!(
        classified_kind("lib/core1.lisp", "", "lisp-дані"),
        "lisp-дані",
        "production Core1 data that drives compatibility must remain enforced"
    );

    assert_eq!(
        rust_nonsemantic_data_kind(
            "crates/xtask/src/compiler_export.rs",
            "",
            "(compiler-semantic-request\n  (identity . ((domain . D3) (bits . 001))))",
        ),
        Some("rust-contract-data"),
        "the canonical compiler export record is contract data, not executable Lisp"
    );
    assert_eq!(
        rust_nonsemantic_data_kind(
            "crates/sens/src/other.rs",
            "",
            "(compiler-semantic-request\n  (identity . car))",
        ),
        None,
        "the same record-shaped text in another production file must not be exempt"
    );
    assert_eq!(
        rust_nonsemantic_data_kind(
            "crates/xtask/src/compiler_export.rs",
            "",
            "(car x)",
        ),
        None,
        "ordinary embedded Lisp in compiler_export remains enforced"
    );

    let gpu_evidence =
        "forbidden_legacy_operation: \"numeric-buffer-map\".to_string()";
    assert_eq!(
        rust_nonsemantic_data_kind(
            "crates/sens/src/gpu_oracle.rs",
            gpu_evidence,
            "numeric-buffer-map",
        ),
        Some("rust-evidence-data"),
        "the forbidden legacy operation name is negative-control evidence"
    );
    assert_eq!(
        rust_nonsemantic_data_kind(
            "crates/sens/src/gpu_oracle.rs",
            "let operation = \"numeric-buffer-map\";",
            "numeric-buffer-map",
        ),
        None,
        "the same surface outside the named negative-control field remains enforced"
    );
    assert_eq!(
        rust_nonsemantic_data_kind(
            "crates/sens/src/other.rs",
            gpu_evidence,
            "numeric-buffer-map",
        ),
        None,
        "the negative-control exemption is path-specific"
    );

    let rust = rust_strings("let a = \"car\"; // \"cdr\"\nlet c = '\"'; let s = r#\"(cons 1 ())\"#;");
    let literals: Vec<&str> = rust.iter().map(|(_, s)| s.as_str()).collect();
    assert_eq!(literals, ["car", "(cons 1 ())"]);
    assert_eq!(rust[1].0, 2);

    let metric_fixture = vec![
        ("rust", "lib/a.rs".to_owned(), 1, "car".to_owned()),
        ("rust-test-instrument", "crates/sens/tests/a.rs".to_owned(), 1, "car".to_owned()),
        ("lisp", "lib/b.lisp".to_owned(), 1, "cdr".to_owned()),
    ];
    assert_eq!(
        debt_metrics(&metric_fixture),
        DebtMetrics { sites: 2, files: 2 },
        "migration debt counts only classifier-enforced production sites"
    );
}
