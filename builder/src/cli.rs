use clap::{Parser, ValueEnum};
use std::{fmt::Display, path::PathBuf};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[arg(short, long, default_value_t = BundleJob::All)]
    pub job: BundleJob,

    /// Bundle specification file
    pub bundle_spec: PathBuf,

    /// Build directory for this bundle
    #[arg(short, long)]
    pub build_dir: PathBuf,

    #[arg(default_value_t = BundleFormat::BundleV1)]
    pub format: BundleFormat,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum BundleJob {
    #[value(name = "all")]
    All,

    #[value(name = "select")]
    Select,

    #[value(name = "pack")]
    Pack,
}

impl Display for BundleJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::All => write!(f, "all"),
            Self::Select => write!(f, "select"),
            Self::Pack => write!(f, "pack"),
        }
    }
}

impl BundleJob {
    pub fn do_select(&self) -> bool {
        matches!(self, Self::All | Self::Select)
    }

    pub fn do_pack(&self) -> bool {
        matches!(self, Self::All | Self::Pack)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum BundleFormat {
    #[value(name = "v1")]
    BundleV1,
}

impl Display for BundleFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BundleV1 => write!(f, "v1")?,
        }
        Ok(())
    }
}
