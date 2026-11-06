use std::{env, path::PathBuf, process::Command};

fn xcrun_output(args: &[&str]) -> String {
    let output = Command::new("xcrun").args(args).output().expect("run xcrun");
    assert!(output.status.success(), "xcrun failed: {args:?}");
    String::from_utf8(output.stdout).expect("UTF-8 xcrun output").trim().into()
}

fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new().app_manifest(
            tauri_build::AppManifest::new().commands(&[
                "get_macos_major_version",
                "get_accessibility_trust_status",
                "request_counter_increment",
                "get_experiment_counter",
            ]),
        ),
    ).expect("build Tauri app manifest");
    println!("cargo:rerun-if-changed=swift/NativeBridge.swift");
    for name in ["DEVELOPER_DIR", "SDKROOT", "MACOSX_DEPLOYMENT_TARGET"] {
        println!("cargo:rerun-if-env-changed={name}");
    }
    assert_eq!(env::var("TARGET").unwrap(), "aarch64-apple-darwin",
        "E2a native bridge currently supports Apple Silicon macOS only");

    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let swiftc = PathBuf::from(xcrun_output(&["--find", "swiftc"]));
    let sdk = xcrun_output(&["--sdk", "macosx", "--show-sdk-path"]);
    let deployment = env::var("MACOSX_DEPLOYMENT_TARGET").unwrap_or_else(|_| "11.0".into());
    let status = Command::new(&swiftc)
        .args(["-parse-as-library", "-emit-library", "-static", "-O",
            "-module-name", "AetherNativeBridge", "-sdk", &sdk,
            "-target", &format!("arm64-apple-macosx{deployment}"), "-module-cache-path"])
        .arg(out.join("swift-module-cache"))
        .arg("swift/NativeBridge.swift")
        .arg("-o").arg(out.join("libaether_native.a"))
        .status().expect("start Swift compilation");
    assert!(status.success(), "Swift native bridge compilation failed");

    // Discover link-time SDK/toolchain paths; no developer path becomes a runtime rpath.
    let toolchain = swiftc.parent().unwrap().parent().unwrap();
    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-search=native={}", toolchain.join("lib/swift/macosx").display());
    println!("cargo:rustc-link-search=native={sdk}/usr/lib/swift");
    println!("cargo:rustc-link-lib=static=aether_native");
    println!("cargo:rustc-link-lib=framework=Foundation");
    println!("cargo:rustc-link-lib=framework=AppKit");
    println!("cargo:rustc-link-lib=framework=ApplicationServices");
    println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
}
