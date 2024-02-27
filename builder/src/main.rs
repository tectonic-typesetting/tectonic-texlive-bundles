use crate::{
    pack::bundlev1::BundleV1,
    select::{picker::FilePicker, spec::BundleSpec},
};
use anyhow::{Context, Result};
use clap::Parser;
use log::LogFormatter;
use std::{
    cmp::Ordering,
    fs::{self, File},
    io::Read,
    thread,
    time::Duration,
};
use tracing::{error, info, warn, Level};

mod cli;
mod log;
mod pack;
mod select;

fn select(cli: &cli::Cli) -> Result<()> {
    let bundle_dir = cli
        .bundle_spec
        .canonicalize()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let mut file = File::open(&cli.bundle_spec)?;
    let mut file_str = String::new();
    file.read_to_string(&mut file_str)?;
    let bundle_config: BundleSpec = match toml::from_str(&file_str) {
        Ok(x) => x,
        Err(e) => {
            error!(
                tectonic_log_source = "select",
                "failed to load bundle specification",
            );
            return Err(e.into());
        }
    };

    if let Err(e) = bundle_config.validate() {
        error!(
            tectonic_log_source = "select",
            "failed to validate bundle specification"
        );
        return Err(e);
    };

    // Remove build dir if it exists
    if cli.build_dir.exists() {
        warn!(
            tectonic_log_source = "select",
            "build dir {} aleady exists",
            cli.build_dir.to_str().unwrap()
        );

        for i in (1..=5).rev() {
            warn!(
                tectonic_log_source = "select",
                "recursively removing {} in {i} second{}",
                cli.build_dir.to_str().unwrap(),
                if i != 1 { "s" } else { "" }
            );
            thread::sleep(Duration::from_secs(1));
        }
        thread::sleep(Duration::from_secs(2));

        fs::remove_dir_all(&cli.build_dir)?;
    }
    fs::create_dir_all(&cli.build_dir).context("while creating build dir")?;

    let mut picker = FilePicker::new(
        bundle_config.clone(),
        cli.build_dir.clone(),
        bundle_dir.clone(),
    )?;

    // Run selector
    let sources: Vec<String> = picker.iter_sources().map(|x| x.to_string()).collect();
    for source in sources {
        picker.add_source(cli, &source)?;
    }
    picker.finish(true)?;

    // Print statistics
    info!(
        tectonic_log_source = "select",
        "summary is below:\n{}",
        picker.stats.make_string()
    );

    match picker.stats.compare_patch_found_applied() {
        Ordering::Equal => {}
        Ordering::Greater => {
            warn!(
                tectonic_log_source = "select",
                "some patches were not applied"
            );
        }
        Ordering::Less => {
            warn!(
                tectonic_log_source = "select",
                "some patches applied multiple times"
            );
        }
    }

    // Check output hash
    {
        let mut file = File::open(cli.build_dir.join("content/SHA256SUM"))?;
        let mut hash = String::new();
        file.read_to_string(&mut hash)?;
        let hash = hash.trim();
        if hash != bundle_config.bundle.expected_hash {
            warn!(
                tectonic_log_source = "select",
                "final bundle hash doesn't match bundle configuration:"
            );
            warn!(tectonic_log_source = "select", "bundle hash is {hash}");
            warn!(
                tectonic_log_source = "select",
                "config hash is {}", bundle_config.bundle.expected_hash
            );
        } else {
            info!(
                tectonic_log_source = "select",
                "final bundle hash matches configuration"
            );
            info!(tectonic_log_source = "select", "hash is {hash}");
        }
    }

    Ok(())
}

fn pack(cli: &cli::Cli) -> Result<()> {
    let mut file = File::open(&cli.bundle_spec)?;
    let mut file_str = String::new();
    file.read_to_string(&mut file_str)?;
    let bundle_config: BundleSpec = toml::from_str(&file_str)?;

    if !cli.build_dir.join("content").is_dir() {
        error!(
            "content directory `{}/content` doesn't exist, can't continue",
            cli.build_dir.to_str().unwrap()
        );
        return Ok(());
    }

    let target_name = format!("{}.ttb", &bundle_config.bundle.name);
    let target = cli.build_dir.join(&target_name);
    if target.exists() {
        if target.is_file() {
            warn!("target bundle `{target_name}` exists, removing");
            fs::remove_file(&target)?;
        } else {
            error!("target bundle `{target_name}` exists and isn't a file, can't continue");
            return Ok(());
        }
    }

    match cli.format {
        cli::BundleFormat::BundleV1 => {
            BundleV1::make(Box::new(File::create(target)?), cli.build_dir.clone())?
        }
    }

    Ok(())
}

#[allow(clippy::needless_return)]
fn main() -> Result<()> {
    let cli = cli::Cli::parse();

    tracing_subscriber::fmt()
        .with_max_level(match cli.log {
            cli::LogLevel::Debug => Level::DEBUG,
            cli::LogLevel::Info => Level::INFO,
            cli::LogLevel::Warn => Level::WARN,
            cli::LogLevel::Error => Level::ERROR,
        })
        .event_format(LogFormatter::new(true))
        .init();

    if cli.job.do_select() {
        match select(&cli) {
            Ok(_) => {}
            Err(e) => {
                error!(
                    tectonic_log_source = "select",
                    "select job failed with error: {e}"
                );
                return Err(e);
            }
        };
    }

    if cli.job.do_pack() {
        match pack(&cli) {
            Ok(_) => {}
            Err(e) => {
                error!(
                    tectonic_log_source = "pack",
                    "bundle packer failed with error: {e}"
                );
                return Err(e);
            }
        };
    }

    Ok(())
}
