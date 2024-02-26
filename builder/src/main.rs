use crate::{
    build::bundlev1::BundleV1,
    select::{picker::FilePicker, spec::BundleSpec},
};
use anyhow::Context;
use clap::{Parser, Subcommand, ValueEnum};
use log::LogFormatter;
use std::{
    cmp::Ordering,
    error::Error,
    fs::{self, File},
    io::Read,
    path::PathBuf,
};
use tracing::{error, info, warn, Level};

mod build;
mod log;
mod select;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Prepare files for a bundle
    Select {
        /// Bundle specification directory
        bundle_dir: PathBuf,

        /// Build directory for this bundle
        build_dir: PathBuf,
    },

    /// Build a bundle
    Build {
        format: BundleFormat,

        /// Bundle specification directory
        bundle_dir: PathBuf,

        /// Build directory for this bundle
        build_dir: PathBuf,
    },
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum BundleFormat {
    #[value(name = "v1")]
    BundleV1,
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .event_format(LogFormatter::new(true))
        .init();

    match cli.command {
        Commands::Select {
            bundle_dir,
            build_dir,
        } => {
            let mut file = File::open(bundle_dir.join("bundle.toml"))?;
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
            let build_dir = build_dir.join("output").join(&bundle_config.bundle.name);
            if build_dir.exists() {
                info!(
                    tectonic_log_source = "select",
                    "removing build dir {build_dir:?}"
                );
                fs::remove_dir_all(&build_dir)?;
            }
            fs::create_dir_all(&build_dir).context("while creating build dir")?;

            /*
                        let source_dir = PathBuf::from("../build/texlive/").join(&bundle_config.texlive_name);


                        // Check input hash
                        {
                            let mut file = File::open(source_dir.join("TEXLIVE-SHA256SUM"))?;
                            let mut hash = String::new();
                            file.read_to_string(&mut hash)?;
                            let hash = hash.trim();
                            if hash != bundle_config.texlive_hash {
                                error!(
                                    tectonic_log_source = "select",
                                    "texlive hash doesn't match, refusing to continue"
                                );
                                return Ok(());
                            }
                        }
            d*/

            let mut picker = FilePicker::new(bundle_config.clone(), build_dir.clone())?;

            let source_dir = PathBuf::from("../build/texlive/texlive-20230313-texmf");
            picker.add_source("include", &bundle_dir.join("include"))?;
            picker.add_source("texlive", &source_dir)?;

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
                        "bundle hash doesn't match bundle.toml!"
                    )
                } else {
                    info!(
                        tectonic_log_source = "select",
                        "bundle hash matches bundle.toml"
                    );
                }
            }
        }
        Commands::Build {
            format,
            bundle_dir,
            build_dir,
        } => {
            let mut file = File::open(bundle_dir.join("bundle.toml"))?;
            let mut file_str = String::new();
            file.read_to_string(&mut file_str)?;
            let bundle_config: BundleSpec = toml::from_str(&file_str)?;

            let build_dir = build_dir.join("output").join(&bundle_config.bundle.name);

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

            match format {
                BundleFormat::BundleV1 => {
                    BundleV1::make(Box::new(File::create(target)?), build_dir)?
                }
            }
        }
    }

    Ok(())
}
