use super::BundleInput;
use anyhow::Result;
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{Read, Seek},
    path::PathBuf,
};
use tar::Archive;
use tracing::info;

pub struct TarBundleInput {
    archive: Archive<File>,
    root: PathBuf,
    hash: String,
}

impl TarBundleInput {
    pub fn new(path: PathBuf, root: Option<PathBuf>) -> Result<Self> {
        let path = path.canonicalize()?;
        let mut file = File::open(&path)?;

        info!(
            tectonic_log_source = "select",
            "computing hash of {}",
            path.to_str().unwrap()
        );

        let hash = {
            let mut hasher = Sha256::new();
            let _ = std::io::copy(&mut file, &mut hasher)?;
            hasher
                .finalize()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<Vec<_>>()
                .concat()
        };

        file.seek(std::io::SeekFrom::Start(0))?;
        Ok(Self {
            archive: Archive::new(file),
            root: root.unwrap_or(PathBuf::from("")),
            hash,
        })
    }

    pub fn hash(&self) -> &str {
        &self.hash
    }
}

impl BundleInput for TarBundleInput {
    fn iter_files(&mut self) -> impl Iterator<Item = Result<(String, Box<dyn Read + '_>)>> + '_ {
        self.archive.entries().unwrap().filter_map(|x| {
            // TODO: error handling
            let xr = x.as_ref().unwrap();

            if !xr.header().entry_type().is_file() {
                None
            } else {
                let path = xr.path().unwrap();

                if !path.starts_with(&self.root) {
                    None
                } else {
                    Some(Ok((
                        path.strip_prefix(&self.root)
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .to_string(),
                        Box::new(x.unwrap()) as Box<dyn Read>,
                    )))
                }
            }
        })
    }
}
