use super::BundleInput;
use anyhow::Result;
use std::{fs::File, io::Read, path::PathBuf};
use tar::Archive;

pub struct TarBundleInput {
    archive: Archive<File>,
    root: PathBuf,
}

impl TarBundleInput {
    pub fn new(path: PathBuf, root: Option<PathBuf>) -> Self {
        Self {
            archive: Archive::new(File::open(&path).unwrap()),
            root: root.unwrap_or(PathBuf::from("")),
        }
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
