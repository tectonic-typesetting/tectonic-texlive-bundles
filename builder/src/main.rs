use crate::{build::bundlev1::BundleV1, select::BundleConfig};
use anyhow::Context;
use clap::{Parser, Subcommand, ValueEnum};
use log::LogFormatter;
use std::{
    error::Error,
    fs::{self, File},
    io::Read,
    path::PathBuf,
};
use tracing::{error, info, warn, Level};

mod build;
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
        bundle_dir: PathBuf,

        /// Build directory for this bundle
        build_dir: PathBuf,
    },

    /// Build a bundle
    Build {
        format: BundleFormat,
        content_dir: PathBuf,
        target: String,
    },
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum BundleFormat {
    #[value(name = "v1")]
    BundleV1,
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Select {
            bundle_dir,
            build_dir,
        } => {
            let mut file = File::open(bundle_dir.join("bundle.toml"))?;
            let mut file_str = String::new();
            file.read_to_string(&mut file_str)?;
            let bundle_config: BundleConfig = toml::from_str(&file_str)?;

            let source_dir = PathBuf::from("../build/texlive/").join(&bundle_config.texlive_name);

            // Remove build dir if it exists
            let build_dir = build_dir.join("output").join(&bundle_config.name);
            if build_dir.exists() {
                info!("removing build dir {build_dir:?}");
                fs::remove_dir_all(&build_dir)?;
            }
            fs::create_dir_all(&build_dir).context("while creating build dir")?;

            // Check input hash
            {
                let mut file = File::open(source_dir.join("TEXLIVE-SHA256SUM"))?;
                let mut hash = String::new();
                file.read_to_string(&mut hash)?;
                let hash = hash.trim();
                if hash != bundle_config.texlive_hash {
                    error!("texlive hash doesn't match, refusing to continue");
                    return Ok(());
                }
            }

            let mut picker = select::FilePicker::new(&bundle_config, build_dir.clone())?;

            if bundle_dir.join("patches").exists() {
                picker.load_diffs_from(&bundle_dir.join("patches"))?;
            }

            picker.add_tree("include", &bundle_dir.join("include"))?;
            picker.add_tree("texlive", &source_dir)?;

            picker.finish(true)?;
            print!("{}", picker.stats.make_string());

            // Check output hash
            {
                let mut file = File::open(build_dir.join("content/SHA256SUM"))?;
                let mut hash = String::new();
                file.read_to_string(&mut hash)?;
                let hash = hash.trim();
                info!("bundle hash is `{hash}`");
                if hash != bundle_config.result_hash {
                    warn!("bundle hash doesn't match bundle.toml!")
                }
            }
        }
        Commands::Build {
            format,
            content_dir,
            target,
        } => match format {
            BundleFormat::BundleV1 => BundleV1::make(Box::new(File::create(target)?), content_dir)?,
        },
    }

    Ok(())
}
