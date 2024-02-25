use crate::build::bundlev1::BundleV1;
use clap::{Parser, Subcommand, ValueEnum};
use std::{error::Error, fs::File, path::PathBuf};

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
        build_dir: PathBuf,
        bundle_texlive_name: String,
        bundle_name: String,
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
            bundle_texlive_name,
            bundle_name,
        } => {
            let mut picker = select::FilePicker::new(&bundle_dir, &build_dir, &bundle_name)?;

            picker.add_extra()?;

            picker.add_tree(
                "texlive",
                &build_dir.join("texlive").join(bundle_texlive_name),
            )?;

            println!("Preparing auxillary files...");
            picker.add_search()?;
            picker.add_meta_files()?;
            picker.generate_debug_files()?;
            picker.show_summary();
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
