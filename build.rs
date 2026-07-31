use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn git_output(manifest_dir: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git").arg("-C").arg(manifest_dir).args(args).output().ok()?;

    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn watch_git_path(manifest_dir: &Path, path: &str) {
    if let Some(path) = git_output(manifest_dir, &["rev-parse", "--git-path", path]) {
        let path = PathBuf::from(path);
        let path = if path.is_absolute() {
            path
        } else {
            manifest_dir.join(path)
        };
        println!("cargo:rerun-if-changed={}", path.display());
    }
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src");

    let manifest_dir = env::var_os("CARGO_MANIFEST_DIR").map(PathBuf::from);
    let Some(manifest_dir) = manifest_dir.filter(|path| path.join(".git").exists()) else {
        return;
    };

    watch_git_path(&manifest_dir, "HEAD");
    watch_git_path(&manifest_dir, "index");
    if let Some(head_ref) = git_output(&manifest_dir, &["symbolic-ref", "-q", "HEAD"]) {
        watch_git_path(&manifest_dir, &head_ref);
    }

    if let Some(revision) = git_output(&manifest_dir, &["rev-parse", "--short=7", "HEAD"]) {
        println!("cargo:rustc-env=TERMINALIST_GIT_REVISION={revision}");
    }

    if git_output(&manifest_dir, &["status", "--porcelain", "--untracked-files=no"])
        .is_some_and(|status| !status.is_empty())
    {
        println!("cargo:rustc-env=TERMINALIST_GIT_DIRTY=1");
    }
}
