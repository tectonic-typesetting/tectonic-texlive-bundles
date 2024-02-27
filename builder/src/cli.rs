use clap::{Parser, ValueEnum};
use std::{fmt::Display, path::PathBuf};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Which job we should run. `all` is default,
    /// but single jobs can be run on their own for debugging.
    #[arg(long, default_value_t = BundleJob::All)]
    pub job: BundleJob,

    /// Bundle specification TOML file.
    pub bundle_spec: PathBuf,

    /// Build directory for this bundle.
    /// Will be removed.
    #[arg(short, long)]
    pub build_dir: PathBuf,

    /// What kind of bundle should we produce?
    /// This only has an effect when running jobs `all` or `pack`
    #[arg(default_value_t = BundleFormat::BundleV1)]
    pub format: BundleFormat,

    /// Log verbosity level.
    #[arg(long, default_value_t = LogLevel::Info)]
    pub log: LogLevel,

    /// If this flag is set, don't fail when an input's hash doesn't match
    /// the hash specified in the bundle's configuration file.
    /// This only has an effect when running jobs `all` or `select`
    #[arg(long, default_value_t = false)]
    pub allow_hash_mismatch: bool,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum LogLevel {
    /// Show extra log messages
    #[value(name = "debug")]
    Debug,

    /// Show some messages and progress
    #[value(name = "info")]
    Info,

    /// Only show warnings
    #[value(name = "warn")]
    Warn,

    /// Only show errors
    #[value(name = "error")]
    Error,
}

impl Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Debug => write!(f, "debug")?,
            Self::Info => write!(f, "info")?,
            Self::Warn => write!(f, "warn")?,
            Self::Error => write!(f, "error")?,
        }
        Ok(())
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum BundleJob {
    /// Run the following jobs in order
    #[value(name = "all")]
    All,

    /// (Stage 1) Select and patch all files in this bundle
    #[value(name = "select")]
    Select,

    /// (Stage 2) Pack selected files into a bundle
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
