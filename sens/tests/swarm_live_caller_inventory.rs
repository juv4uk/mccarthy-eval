use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

// Complete retired coordination vocabulary that used to be multiplexed onto
// the sens :9999 semantic oracle. These names may still exist inside the
// independent swarm-node protocol; that crate is deliberately excluded below.
const RETIRED_COORDINATION_OPS: &[&str] = &[
    "hello",
    "heartbeat",
    "claim",
    "release",
    "complete-task",
    "define-task",
    "validate-tasks",
    "sync-tasks",
    "sync-milestone",
    "next-best-action",
    "list-task-state",
    "list-tasks",
    "presence",
    "list-claims",
    "capability-request",
    "subscribe",
    "publish",
    "notify",
    "poll",
];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn scan_tree(root: &Path, dir: &Path, hits: &mut BTreeMap<String, Vec<String>>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let rel = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            // These operation names are legitimate on the new coordination
            // authority itself. C5 only forbids them on sens :9999.
            if rel == "crates/swarm-node" || rel.starts_with("crates/swarm-node/") {
                continue;
            }
            scan_tree(root, &path, hits);
            continue;
        }

        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");

        let executable_surface = matches!(
            path.extension().and_then(|ext| ext.to_str()),
            Some("rs" | "py" | "sh" | "wsm" | "my")
        );
        if !executable_surface {
            continue;
        }

        let Ok(source) = fs::read_to_string(&path) else {
            continue;
        };

        for op in RETIRED_COORDINATION_OPS {
            let needle = format!("(op {op})");
            if source.contains(&needle) {
                hits.entry(rel.clone()).or_default().push((*op).to_string());
            }
        }
    }
}

fn inventory() -> BTreeMap<String, Vec<String>> {
    let root = repo_root();
    let mut hits = BTreeMap::new();

    scan_tree(&root, &root.join("crates"), &mut hits);
    scan_tree(&root, &root.join("scripts"), &mut hits);

    for ops in hits.values_mut() {
        ops.sort();
        ops.dedup();
    }
    hits
}

fn is_compatibility_test(path: &str) -> bool {
    path.starts_with("crates/") && path.contains("/tests/")
}

#[test]
fn no_production_or_operational_retired_coordination_callers_remain() {
    let live: BTreeMap<_, _> = inventory()
        .into_iter()
        .filter(|(path, _)| !is_compatibility_test(path))
        .collect();

    assert!(
        live.is_empty(),
        "C5 post-removal gate failed; retired :9999 production/operational surface remains: {live:?}"
    );
}

#[test]
fn no_retired_coordination_compatibility_callers_remain() {
    let compatibility: BTreeMap<_, _> = inventory()
        .into_iter()
        .filter(|(path, _)| is_compatibility_test(path))
        .collect();

    assert!(
        compatibility.is_empty(),
        "C5 post-removal gate requires zero retired :9999 compatibility callers: {compatibility:?}"
    );
}
