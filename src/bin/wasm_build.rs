use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    let status = Command::new("cargo")
        .args(["build", "--target", "wasm32-unknown-unknown", "--release"])
        .env("RUSTFLAGS", "-C link-arg=--allow-undefined")
        .status()
        .expect("Error: Failed to execute cargo build command");

    if !status.success() {
        eprintln!("Error: Cargo build failed");
        std::process::exit(1);
    }

    let src = Path::new("target/wasm32-unknown-unknown/release/suiktiles-game.wasm");
    let dest = Path::new("web/suiktiles-game.wasm");

    if !dest.exists() {
        fs::create_dir_all(dest.parent().unwrap())
            .expect("Error: Failed to create destination directory");
    }

    fs::copy(src, dest).expect("Error: Failed to copy wasm file to destination");
}
