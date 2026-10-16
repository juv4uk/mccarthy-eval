use sens::capability_installed;
use std::fs;
use std::path::{Path, PathBuf};

fn collect_rust_sources(root: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(root).expect("core source directory must be readable") {
        let entry = entry.expect("source directory entry must be readable");
        let path = entry.path();
        if path.is_dir() {
            collect_rust_sources(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

#[test]
fn concrete_tcp_types_are_confined_to_value_storage_not_core_operations() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let src = manifest.join("src");
    let mut files = Vec::new();
    collect_rust_sources(&src, &mut files);
    files.sort();

    let mut concrete_type_files = Vec::new();
    let forbidden_operations = [
        "TcpStream::connect",
        "TcpListener::bind",
        ".accept()",
        ".shutdown(",
        ".write_all(",
    ];

    for path in files {
        let text = fs::read_to_string(&path).expect("Rust source must be UTF-8");
        let relative = path
            .strip_prefix(&src)
            .expect("source path must stay under crate src")
            .to_string_lossy()
            .replace('\\', "/");

        if text.contains("net::TcpStream") || text.contains("net::TcpListener") {
            concrete_type_files.push(relative.clone());
        }

        for operation in forbidden_operations {
            assert!(
                !text.contains(operation),
                "core must not own TCP OS operation {operation:?}; found in {relative}"
            );
        }
    }

    assert_eq!(
        concrete_type_files,
        vec!["value.rs"],
        "concrete std::net TCP types must remain confined to the Value storage boundary"
    );
}

#[test]
fn bare_core_installs_no_tcp_capabilities() {
    for name in [
        "tcp-connect",
        "tcp-listen-raw",
        "tcp-accept",
        "tcp-read-raw",
        "tcp-write-raw",
        "tcp-close",
    ] {
        assert!(
            !capability_installed(name),
            "bare sens core unexpectedly installed host capability {name}"
        );
    }
}
