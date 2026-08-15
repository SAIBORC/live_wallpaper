use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=app.ico");
    println!("cargo:rerun-if-changed=app.rc");

    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let ico = root.join("app.ico");
    if !ico.exists() {
        println!("cargo:warning=app.ico not found; icon not embedded");
        return;
    }
    let rc = root.join("app.rc");
    if !rc.exists() {
        println!("cargo:warning=app.rc not found; icon not embedded");
        return;
    }

    let Some(rc_exe) = find_rc() else {
        println!("cargo:warning=rc.exe not found; icon not embedded");
        return;
    };

    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let res_path = out_dir.join("app.res");

    let status = Command::new(&rc_exe)
        .arg("/nologo")
        .arg("/fo")
        .arg(&res_path)
        .arg(&rc)
        .status()
        .expect("run rc.exe");
    if !status.success() {
        println!("cargo:warning=rc.exe failed; icon not embedded");
        return;
    }

    println!("cargo:rustc-link-arg={}", res_path.display());
}

fn find_rc() -> Option<std::path::PathBuf> {
    let kits = r"C:\Program Files (x86)\Windows Kits\10\bin";
    let base = std::path::Path::new(kits);
    if base.exists() {
        let mut dirs: Vec<_> = std::fs::read_dir(base)
            .ok()?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .collect();
        dirs.sort();
        for dir in dirs.into_iter().rev() {
            for arch in ["x64", "x86", "arm64"] {
                let cand = dir.join(arch).join("rc.exe");
                if cand.exists() {
                    return Some(cand);
                }
            }
        }
    }
    None
}
