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
                "failed to load bundle specification: {}",
                e.message()
            );
            return Ok(());
        }
    };

    if let Err(e) = bundle_config.validate() {
        error!(
            tectonic_log_source = "select",
            "failed to validate bundle specification: {e}"
        );
        return Ok(());
    };

    // Remove build dir if it exists
    let build_dir = cli
        .build_dir
        .join("output")
        .join(&bundle_config.bundle.name);
    if build_dir.exists() {
        info!(
            tectonic_log_source = "select",
            "removing build dir {build_dir:?}"
        );
        fs::remove_dir_all(&build_dir)?;
    }
    fs::create_dir_all(&build_dir).context("while creating build dir")?;

    let mut picker = FilePicker::new(bundle_config.clone(), build_dir.clone(), bundle_dir.clone())?;

    picker.add_source("include")?;
    picker.add_source("texlive")?;

    picker.finish(true)?;
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
        let mut file = File::open(build_dir.join("content/SHA256SUM"))?;
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

    let build_dir = cli
        .build_dir
        .join("output")
        .join(&bundle_config.bundle.name);

    if !build_dir.join("content").is_dir() {
        error!("content directory `{build_dir:?}/content` doesn't exist, can't continue");
        return Ok(());
    }

    let target_name = format!("{}.ttb", &bundle_config.bundle.name);
    let target = build_dir.join(&target_name);
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
        cli::BundleFormat::BundleV1 => BundleV1::make(Box::new(File::create(target)?), build_dir)?,
    }

    Ok(())
}

fn main() -> Result<()> {
    let cli = cli::Cli::parse();

    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .event_format(LogFormatter::new(true))
        .init();

    if cli.job.do_select() {
        select(&cli)?;
    }

    if cli.job.do_pack() {
        pack(&cli)?;
    }

    Ok(())
}
