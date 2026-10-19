//! Shared loading and argument parsing for standalone GPUI hosts.
//!
//! Hosts retain their application initialization and window/root lifecycle.
//! These helpers only prepare NSH sources and options; supplied component
//! catalogs are preserved without introducing a window or a webview dependency.

use std::path::{Path, PathBuf};

use crate::{AppOptions, AppSource, Appearance, AssetBase, Bundle};

/// Finds NSH resources relative to an installed release executable.
pub fn resources(executable: &Path) -> PathBuf {
    let directory = executable.parent().unwrap_or(Path::new("."));
    if directory.file_name().is_some_and(|name| name == "MacOS")
        && let Some(contents) = directory.parent()
        && contents.file_name().is_some_and(|name| name == "Contents")
    {
        contents.join("Resources/nsh")
    } else {
        directory.join("nsh")
    }
}

/// Reads the installed bundle and metadata, retaining the host's component catalog.
/// Honors `NSH_APP_DIR`, otherwise uses the executable's native package layout.
pub fn package_source(
    mut options: AppOptions,
) -> Result<(AppSource, AppOptions), Box<dyn std::error::Error>> {
    let directory = match std::env::var_os("NSH_APP_DIR") {
        Some(directory) => PathBuf::from(directory),
        None => resources(&std::env::current_exe()?.canonicalize()?),
    };
    // SAFETY: this is the bytecode shipped in the application's own resources.
    let bundle = unsafe { Bundle::read_trusted(&directory.join("bundle.nshc")) }?;
    let metadata: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.join("app.json"))?)?;
    options.asset_base = Some(AssetBase::Directory(directory.join("assets")));
    if let Some(name) = metadata.get("name").and_then(|value| value.as_str()) {
        options.title = name.to_owned();
    }
    if let Some(id) = metadata.get("id").and_then(|value| value.as_str()) {
        options.app_id = id.to_owned();
    }
    options.appearance = match metadata.get("appearance").and_then(|value| value.as_str()) {
        Some("light") => Appearance::Light,
        Some("dark") => Appearance::Dark,
        _ => Appearance::System,
    };
    Ok((AppSource::Bundle(bundle), options))
}

/// Parses the development host protocol strictly, retaining host-provided options.
/// Missing values, unsupported arguments and non-positive dimensions are errors.
#[cfg(feature = "dev")]
pub fn development_source(
    arguments: impl IntoIterator<Item = String>,
    mut options: AppOptions,
) -> Result<(AppSource, AppOptions), Box<dyn std::error::Error>> {
    let mut arguments = arguments.into_iter();
    let mut source = None;
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--dev-server" => {
                let url = arguments.next().ok_or("--dev-server needs a URL")?;
                if !(url.starts_with("ws://") || url.starts_with("wss://")) {
                    return Err("--dev-server needs a ws:// or wss:// URL".into());
                }
                source = Some(AppSource::DevServer(url));
            }
            "--bundle" => {
                let path = PathBuf::from(arguments.next().ok_or("--bundle needs a path")?);
                // SAFETY: the developer explicitly selected this local bundle.
                let bundle = unsafe { Bundle::read_trusted(&path) }?;
                options.asset_base = path
                    .parent()
                    .map(|directory| AssetBase::Directory(directory.join("assets")));
                source = Some(AppSource::Bundle(bundle));
            }
            "--trace" => options.trace = true,
            "--fps" => options.fps = true,
            "--size" => {
                let size = arguments.next().ok_or("--size needs WIDTHxHEIGHT")?;
                let (width, height) = size.split_once('x').ok_or("--size needs WIDTHxHEIGHT")?;
                options.size = (width.parse()?, height.parse()?);
                if options.size.0 <= 0 || options.size.1 <= 0 {
                    return Err("--size dimensions must be positive".into());
                }
            }
            _ => return Err(format!("unknown host argument: {argument}").into()),
        }
    }
    let source =
        source.unwrap_or_else(|| AppSource::DevServer("ws://127.0.0.1:8790/ws".to_owned()));
    Ok((source, options))
}

/// Installs the release host's stderr logger for warnings and JavaScript logs.
pub fn init_logging() {
    struct Logger;
    impl log::Log for Logger {
        fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
            metadata.level() <= log::Level::Warn || metadata.target() == "js"
        }
        fn log(&self, record: &log::Record<'_>) {
            if self.enabled(record.metadata()) {
                eprintln!("{} {}: {}", record.level(), record.target(), record.args());
            }
        }
        fn flush(&self) {}
    }
    static LOGGER: Logger = Logger;
    if log::set_logger(&LOGGER).is_ok() {
        log::set_max_level(log::LevelFilter::Info);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_resources_follow_the_native_nsh_layout() {
        assert_eq!(
            resources(Path::new("/opt/Counter/Counter")),
            Path::new("/opt/Counter/nsh")
        );
        assert_eq!(
            resources(Path::new(
                "/Applications/Counter.app/Contents/MacOS/Counter"
            )),
            Path::new("/Applications/Counter.app/Contents/Resources/nsh")
        );
        assert_eq!(
            resources(Path::new("/home/me/MacOS/Counter")),
            Path::new("/home/me/MacOS/nsh")
        );
    }

    #[cfg(feature = "dev")]
    #[test]
    fn development_arguments_preserve_host_options() {
        let options = AppOptions {
            title: "Custom host".into(),
            app_id: "test.custom.host".into(),
            ..Default::default()
        };
        let (source, options) = development_source(
            [
                "--dev-server",
                "wss://localhost/ws",
                "--size",
                "900x600",
                "--trace",
                "--fps",
            ]
            .into_iter()
            .map(str::to_owned),
            options,
        )
        .unwrap();
        assert!(matches!(source, AppSource::DevServer(url) if url == "wss://localhost/ws"));
        assert_eq!(options.title, "Custom host");
        assert_eq!(options.app_id, "test.custom.host");
        assert_eq!(options.size, (900, 600));
        assert!(options.trace && options.fps);
    }

    #[cfg(feature = "dev")]
    #[test]
    fn invalid_development_arguments_fail_before_opening_a_window() {
        for arguments in [
            vec!["--dev-server"],
            vec!["--dev-server", "https://example.com"],
            vec!["--size", "0x800"],
            vec!["--size", "1200"],
            vec!["--bundle"],
            vec!["--unknown"],
        ] {
            assert!(
                development_source(
                    arguments.into_iter().map(str::to_owned),
                    AppOptions::default()
                )
                .is_err()
            );
        }
    }
}
