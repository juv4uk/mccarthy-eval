//! Build the sterile Zig corridor and link it into the host.
//! Zig owns the physical transport; Rust only calls the C ABI (v2).
use std::{env, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=zig/transport_abi.zig");
    println!("cargo:rerun-if-changed=zig/t5.zig");
    println!("cargo:rerun-if-changed=zig/f3c.zig");
    println!("cargo:rerun-if-changed=zig/transport/e1.zig");
    println!("cargo:rerun-if-changed=zig/transport/e1_writer.zig");
    println!("cargo:rerun-if-changed=zig/transport/fields.zig");
    println!("cargo:rerun-if-env-changed=ZIG");

    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    let archive = out.join("libsens_zig_transport.a");
    let compiler = env::var_os("ZIG").unwrap_or_else(|| "zig".into());

    // Refuse toolchain drift: the version is pinned in .zig-version.
    let pinned = std::fs::read_to_string(manifest.join(".zig-version")).expect(".zig-version");
    let pinned = pinned.trim();
    let version = Command::new(&compiler).arg("version").output()
        .expect("sens-host requires the pinned Zig compiler");
    assert!(version.status.success(), "sens-host: Zig version probe failed");
    let actual = std::str::from_utf8(&version.stdout).expect("utf8").trim();
    assert_eq!(actual, pinned, "sens-host: wrong Zig version");

    let status = Command::new(&compiler)
        .current_dir(&manifest)
        .arg("build-lib").arg("zig/transport_abi.zig")
        .arg("-O").arg("ReleaseFast").arg("-lc").arg("-fPIC")
        .arg(format!("-femit-bin={}", archive.display()))
        .status().expect("sens-host: Zig compiler unavailable");
    assert!(status.success(), "sens-host: Zig C ABI compile failed");

    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=sens_zig_transport");
}
