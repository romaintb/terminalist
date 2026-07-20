use std::env;
use std::path::Path;
use std::process::Command;

fn git_output(manifest_dir: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git").arg("-C").arg(manifest_dir).args(args).output().ok()?;

    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn main() {
    let manifest_dir = env::var_os("CARGO_MANIFEST_DIR").map(std::path::PathBuf::from);
    let Some(manifest_dir) = manifest_dir.filter(|path| path.join(".git").exists()) else {
        return;
    };

    if let Some(revision) = git_output(&manifest_dir, &["rev-parse", "--short=7", "HEAD"]) {
        println!("cargo:rustc-env=TERMINALIST_GIT_REVISION={revision}");
    }

    if git_output(&manifest_dir, &["status", "--porcelain", "--untracked-files=no"])
        .is_some_and(|status| !status.is_empty())
    {
        println!("cargo:rustc-env=TERMINALIST_GIT_DIRTY=1");
    }
}
