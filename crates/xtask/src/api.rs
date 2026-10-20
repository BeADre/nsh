//! `cargo xtask api`: records the API this checkout offers apps in
//! `api/runtime.txt` and `api/types/` (the engine's `compat` tests compare
//! them with what the runtime makes). The record is append-only, as the web
//! platform's is: what a merged change recorded stays, and nothing is frozen
//! per release.
//!
//! `--keep <app>...` adds fixtures, `api/apps/<app>-<version>/`: example
//! apps as this checkout's `nsh build --release` writes them, for when the
//! JSX compiler's output changes (new `nsh:rt` calls), so the record keeps
//! what the new compiler writes next to what the older ones wrote
//! (docs/dev/compatibility.md).

use std::{
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context as _, bail};

use crate::{android::root, term};

pub(crate) fn api(keep: &[String]) -> anyhow::Result<()> {
    let root = root();
    term::cargo(
        Command::new("cargo")
            .current_dir(&root)
            .args(["test", "-p", "nsh-engine", "--test", "engine", "--"])
            .args(["--exact", "compat::surface_is_recorded"])
            .env("NSH_API_UPDATE", "1"),
        "and recording the API",
    )?;
    term::done("recorded api/runtime.txt and api/types");
    if keep.is_empty() {
        return Ok(());
    }
    let apps = root.join("api/apps");
    let version = env!("CARGO_PKG_VERSION");
    // Check every app first, so a refused one leaves no fixture half added.
    let kept: Vec<_> = keep
        .iter()
        .map(|app| {
            let dir = apps.join(format!("{app}-{version}"));
            if dir.exists() {
                // Fixtures are what apps built then call: never rewritten.
                bail!("{}: already kept", shown(&root, &dir));
            }
            if !root.join("examples").join(app).is_dir() {
                bail!("examples/{app}: no such app");
            }
            Ok((app, dir))
        })
        .collect::<anyhow::Result<_>>()?;
    term::cargo(
        Command::new("cargo")
            .current_dir(&root)
            .args(["build", "-p", "nsh-cli"]),
        "nsh",
    )?;
    for (app, dir) in kept {
        let out = root.join("target/api").join(app);
        let _ = std::fs::remove_dir_all(&out);
        term::run(
            Command::new(root.join("target/debug/nsh"))
                .current_dir(&root)
                .args(["build", "--release", "-o"])
                .arg(&out)
                .arg(Path::new("examples").join(app)),
        )?;
        // The script comes before the new fixture exists, so it is the
        // newest older one's.
        let previous = newest_fixture(&apps, app)?;
        std::fs::create_dir_all(&dir)?;
        std::fs::copy(out.join("bundle.js"), dir.join("bundle.js"))
            .with_context(|| format!("{app}'s bundle"))?;
        if out.join("assets").exists() {
            copy_dir(&out.join("assets"), &dir.join("assets"))?;
        }
        let script = dir.join("script.txt");
        match previous.map(|p| p.join("script.txt")) {
            Some(from) if from.exists() => {
                std::fs::copy(from, &script)?;
            }
            _ => {
                std::fs::write(&script, "# What a user does and sees (compat/apps.rs).\n")?;
                term::warn(format!(
                    "{}: write what a user does and sees",
                    shown(&root, &script)
                ));
            }
        }
        term::done(format!("kept {}", shown(&root, &dir)));
    }
    term::done(
        "run `cargo test -p nsh-engine --test engine compat::apps_still_run` to check the fixtures",
    );
    Ok(())
}

fn shown(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn copy_dir(from: &Path, to: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let path = entry?.path();
        let dest = to.join(path.file_name().unwrap());
        if path.is_dir() {
            copy_dir(&path, &dest)?;
        } else {
            std::fs::copy(&path, &dest)?;
        }
    }
    Ok(())
}

/// The fixture of `app` with the latest version: `<app>-<version>`, where a
/// bare `<app>` (the first fixtures, kept before they had versions) is the
/// oldest.
fn newest_fixture(apps: &Path, app: &str) -> anyhow::Result<Option<PathBuf>> {
    if !apps.exists() {
        return Ok(None);
    }
    let version = |name: &str| -> Option<Vec<u32>> {
        if name == app {
            return Some(Vec::new());
        }
        // `gallery-rs` is not a version of `gallery`: only numbers count.
        name.strip_prefix(app)?
            .strip_prefix('-')?
            .split('.')
            .map(|p| p.parse().ok())
            .collect()
    };
    let mut fixtures: Vec<_> = std::fs::read_dir(apps)?
        .filter_map(|e| {
            let path = e.ok()?.path();
            let v = version(&path.file_name()?.to_string_lossy())?;
            path.is_dir().then_some((v, path))
        })
        .collect();
    fixtures.sort();
    Ok(fixtures.pop().map(|(_, p)| p))
}
