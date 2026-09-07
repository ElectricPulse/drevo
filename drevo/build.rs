use std::{env, process::Command};

fn main() {
    println!("cargo:rerun-if-env-changed=RUSTC");
    println!("cargo:rustc-check-cfg=cfg(drevo_nightly)");

    let rustc = env::var_os("RUSTC").expect("Cargo must set RUSTC");
    let output = Command::new(rustc)
        .arg("--version")
        .output()
        .expect("failed to run rustc --version");
    let version = String::from_utf8_lossy(&output.stdout);

    if version.contains("nightly") {
        println!("cargo:rustc-cfg=drevo_nightly");
    }
}
