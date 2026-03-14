use std::process::Command;

fn main() {
    let commit = cmd("git", &["rev-parse", "--short", "HEAD"]);
    let date = cmd("date", &["+%Y-%m-%d %H:%M:%S %Z"]);
    let host = cmd("hostname", &["-s"]);

    println!("cargo:rustc-env=BUILD_COMMIT={commit}");
    println!("cargo:rustc-env=BUILD_HOST={host}");
    println!("cargo:rustc-env=BUILD_TIME={date}");
}

fn cmd(program: &str, args: &[&str]) -> String {
    Command::new(program)
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_default()
        .trim()
        .to_string()
}
