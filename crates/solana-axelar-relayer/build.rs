//! store the git commit hash as a static env variable that's accessible at build time via env!()
//! macro

fn main() {
    // Execute `git rev-parse --short HEAD` and store it in GIT_ENV  variable
    let git_hash = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .expect("Failed to execute git command")
        .stdout;
    let git_hash = core::str::from_utf8(&git_hash).expect("hash is not a valid utf8");
    println!("cargo:rustc-env=GIT_HASH={git_hash}");
}
