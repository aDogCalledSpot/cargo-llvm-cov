macro_rules! trace {
    ($($tt:tt)*) => {};
}
macro_rules! debug {
    ($($tt:tt)*) => {};
}

#[path = "../../src/fs.rs"]
mod fs;
#[macro_use]
#[path = "../../src/process.rs"]
mod process;

use std::{
    env,
    sync::atomic::{AtomicUsize, Ordering::Relaxed},
};

use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};
use once_cell::sync::Lazy;
use tempfile::{Builder, TempDir};
use walkdir::WalkDir;

static FIXTURES_PATH: Lazy<Utf8PathBuf> =
    Lazy::new(|| Utf8Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"));

#[allow(single_use_lifetimes)]
#[track_caller]
pub(crate) fn cargo_llvm_cov<'a>(
    model: &str,
    name: &str,
    extension: &str,
    args: impl AsRef<[&'a str]>,
) -> Result<()> {
    let workspace_root = test_project(model, name)?;
    let manifest_path = workspace_root.path().join("Cargo.toml").display().to_string();
    let output_dir = FIXTURES_PATH.join("coverage-reports").join(model);
    fs::create_dir_all(&output_dir)?;
    let output_path = output_dir.join(name).with_extension(extension);
    process!(
        env!("CARGO_BIN_EXE_cargo-llvm-cov"),
        "llvm-cov",
        "--color",
        "never",
        "--manifest-path",
        &manifest_path,
        "--output-path",
        output_path.as_str()
    )
    .args(args.as_ref())
    .env_remove("RUST_LOG")
    .run()?;
    if env::var_os("CI").is_some() {
        process!("git", "--no-pager", "diff", "--exit-code", output_path).run()?;
    }
    Ok(())
}

fn test_project(model: &str, name: &str) -> Result<TempDir> {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    let tmpdir = Builder::new()
        .prefix(&format!("test_project_{}_{}_{}", model, name, COUNTER.fetch_add(1, Relaxed)))
        .tempdir()?;
    let workspace_root = tmpdir.path();
    let model_path = FIXTURES_PATH.join("coverage").join(model);

    for entry in WalkDir::new(&model_path).into_iter().filter_map(Result::ok) {
        let from = entry.path();
        let to = &workspace_root.join(from.strip_prefix(&model_path)?);
        if from.is_dir() {
            fs::create_dir(to)?;
        } else {
            fs::copy(from, to)?;
        }
    }

    Ok(tmpdir)
}
