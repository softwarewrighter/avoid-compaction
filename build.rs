use std::process::Command;

fn main() {
    let commit = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_default();

    let date = Command::new("date")
        .args(["+%Y-%m-%d %H:%M"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_default();

    println!(
        "cargo:rustc-env=BUILD_VERSION={} {}",
        commit.trim(),
        date.trim()
    );
}
