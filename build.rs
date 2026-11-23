use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=assets/app.rc");
    println!("cargo:rerun-if-changed=assets/Icon.ico");

    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let Some(rc_exe) = find_rc_exe() else {
        println!("cargo:warning=rc.exe not found; skipping Windows EXE icon embedding");
        return;
    };

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let res_path = out_dir.join("atlas_icon.res");

    let status = Command::new(&rc_exe)
        .args([
            "/nologo",
            "/fo",
            res_path.to_str().unwrap(),
            "assets/app.rc",
        ])
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("cargo:rustc-link-arg-bins={}", res_path.display());
        }
        other => {
            println!("cargo:warning=rc.exe failed ({other:?}); continuing without EXE icon");
        }
    }
}

fn find_rc_exe() -> Option<PathBuf> {
    if let Ok(p) = Command::new("where").arg("rc.exe").output() {
        if p.status.success() {
            if let Some(first) = String::from_utf8_lossy(&p.stdout).lines().next() {
                let pb = PathBuf::from(first.trim());
                if pb.exists() {
                    return Some(pb);
                }
            }
        }
    }

    let kits_bin = Path::new(r"C:\Program Files (x86)\Windows Kits\10\bin");
    if let Ok(entries) = std::fs::read_dir(kits_bin) {
        let mut versions: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path().join("x64").join("rc.exe"))
            .filter(|p| p.exists())
            .collect();
        versions.sort();
        if let Some(latest) = versions.pop() {
            return Some(latest);
        }
    }
    None
}
