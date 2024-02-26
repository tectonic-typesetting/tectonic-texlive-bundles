use anyhow::{bail, Context, Result};
use regex::Regex;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    cmp::Ordering,
    collections::HashMap,
    fmt::Display,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use tracing::{debug, info, trace, warn};
use walkdir::WalkDir;

#[derive(Deserialize)]
pub struct BundleConfig {
    /// The bundle's name
    pub name: String,

    /// The name of the texlive tarball
    pub texlive_name: String,

    /// The hash of the texlive tarball this
    /// bundle is built from
    pub texlive_hash: String,

    /// The hash of the resulting ttbv1 bundle
    pub result_hash: String,

    /// Search paths for this bundle
    pub search_order: Vec<String>,

    /// Files to ignore in this bundle
    pub ignore: Vec<String>,
}

#[derive(Default)]
pub struct PickStatistics {
    /// Total number of files added from each source
    added: HashMap<String, usize>,

    /// Number of extra files added
    extra: usize,

    /// Number of extra file conflicts
    extra_conflict: usize,

    /// Total number of files ignored
    ignored: usize,

    /// Total number of files replaced
    replaced: usize,

    /// Total number of patches applied
    patch_applied: usize,

    /// Total number of patches found
    patch_found: usize,
}

impl PickStatistics {
    /// Returns a pretty status summary string
    pub fn make_string(&self) -> String {
        let mut output_string = format!(
            concat!(
                "=============== Summary ===============\n",
                "    extra file conflicts: {}\n",
                "    files ignored:        {}\n",
                "    files replaced:       {}\n",
                "    diffs applied/found:  {}/{}\n",
                "    =============================\n",
                "    extra files:          {}\n",
            ),
            self.extra_conflict,
            self.ignored,
            self.replaced,
            self.patch_applied,
            self.patch_found,
            self.extra,
        );

        let mut sum = self.extra;
        for (source, count) in &self.added {
            let s = format!("{source} files: ");
            output_string.push_str(&format!("    {s}{}{count}\n", " ".repeat(22 - s.len())));
            sum += count;
        }
        output_string.push_str(&format!("    total files:          {sum}\n\n"));

        output_string.push_str(&format!("{}", "=".repeat(39)));
        output_string
    }

    /// Did we find as many, fewer, or more patches than we applied?
    pub fn compare_patch_found_applied(&self) -> Ordering {
        self.patch_found.cmp(&self.patch_applied)
    }
}

struct FileListEntry {
    /// Path relative to content dir (does not start with a slash)
    path: PathBuf,
    hash: Option<String>,
}

impl Display for FileListEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        format!(
            "/{} {}",
            self.path.to_str().unwrap(),
            match &self.hash {
                Some(s) => &s,
                None => "nohash",
            }
        )
        .fmt(f)
    }
}

pub struct FilePicker {
    /// Where to place this bundle's files
    build_dir: PathBuf,

    /// A compiled list of filename patterns to ignore
    ignore_patterns: Vec<Regex>,

    /// This file picker's statistics
    pub stats: PickStatistics,

    /// All files we've picked so far.
    /// This map's keys are the `path` value of `FileListEntry`.
    filelist: HashMap<PathBuf, FileListEntry>,

    diffs: HashMap<PathBuf, PathBuf>,
    search: Vec<String>,
}

impl FilePicker {
    /// Transform a search order file with shortcuts
    /// (bash-like brace expansion, like `/a/b/{tex,latex}/c`)
    /// into a plain list of strings.
    fn expand_search_line(s: &str) -> Result<Vec<String>> {
        if !(s.contains('{') || s.contains('}')) {
            return Ok(vec![s.to_owned()]);
        }

        let first = match s.find('{') {
            Some(x) => x,
            None => bail!("Bad search path format"),
        };

        let last = match s.find('}') {
            Some(x) => x,
            None => bail!("Bad search path format"),
        };

        let head = &s[..first];
        let mid = &s[first + 1..last];

        if mid.contains('{') || mid.contains('}') {
            // Mismatched or nested braces
            bail!("Bad search path format");
        }

        // We find the first brace, so only tail may have other expansions.
        let tail = Self::expand_search_line(&s[last + 1..s.len()])?;

        if mid.is_empty() {
            bail!("Bad search path format");
        }

        let mut output: Vec<String> = Vec::new();
        for m in mid.split(',') {
            for t in &tail {
                if m.is_empty() {
                    bail!("Bad search path format");
                }
                output.push(format!("{}{}{}", head, m, t));
            }
        }

        Ok(output)
    }

    /// Should we ignore the given file?
    fn ignore_file(&self, source: &str, file_rel_path: &str) -> bool {
        let f = format!("/{source}/{file_rel_path}");
        for pattern in &self.ignore_patterns {
            if pattern.is_match(&f) {
                return true;
            }
        }

        false
    }

    /// Patch a file in-place.
    /// This should be done after calling `add_file`.
    fn apply_patch(&mut self, path: &Path) -> Result<bool> {
        // path is absolute, but self.diffs is indexed by
        // paths relative to content dir.
        let path_rel = path
            .strip_prefix(&self.build_dir.join("content"))
            .context("tried to patch file outside of build direcory")?;

        // Is this file patched?
        if !self.diffs.contains_key(path_rel) {
            return Ok(false);
        }

        info!(
            tectonic_log_source = "select",
            "patching `{}`",
            path_rel.to_str().unwrap()
        );
        self.stats.patch_applied += 1;

        // Discard first line of diff
        let diff_file = fs::read_to_string(&self.diffs[path_rel]).unwrap();
        let (_, diff) = diff_file.split_once('\n').unwrap();

        let mut child = Command::new("patch")
            .arg("--quiet")
            .arg("--no-backup")
            .arg(path)
            .stdin(Stdio::piped())
            .spawn()
            .context("while spawning `patch`")?;

        let mut stdin = child.stdin.take().unwrap();
        stdin
            .write_all(diff.as_bytes())
            .context("while passing diff to `patch`")?;
        drop(stdin);
        child.wait().context("while waiting for `patch`")?;

        Ok(true)
    }

    /// Add a file into the file list.
    fn add_to_filelist(&mut self, path: PathBuf, file: Option<&Path>) -> Result<()> {
        trace!(
            tectonic_log_source = "select",
            "adding `{path:?}` to file list"
        );

        self.filelist.insert(
            path.clone(),
            FileListEntry {
                path,
                hash: match file {
                    None => None,
                    Some(f) => {
                        let mut hasher = Sha256::new();
                        let _ = std::io::copy(&mut fs::File::open(f)?, &mut hasher)?;
                        Some(
                            hasher
                                .finalize()
                                .iter()
                                .map(|b| format!("{b:02x}"))
                                .collect::<Vec<_>>()
                                .concat(),
                        )
                    }
                },
            },
        );

        Ok(())
    }

    /// Add a file to this picker's content directory
    fn add_file(&mut self, path: &Path, source: &str, file_rel_path: &str) -> Result<()> {
        let target_path = self
            .build_dir
            .join("content")
            .join(source)
            .join(file_rel_path);

        // Path to this file, relative to content dir
        let rel = target_path
            .strip_prefix(&self.build_dir.join("content"))
            .unwrap()
            .to_path_buf();

        trace!(
            tectonic_log_source = "select",
            "adding {path:?} from source `{source}`"
        );

        // Skip files that already exist
        if self.filelist.contains_key(&rel) {
            self.stats.extra_conflict += 1;
            warn!(
                tectonic_log_source = "select",
                "{path:?} from source `{source}` already exists, skipping"
            );
            return Ok(());
        }

        fs::create_dir_all(match target_path.parent() {
            Some(x) => x,
            None => bail!("couldn't get parent of target"),
        })
        .context("failed to create content directory")?;

        // Copy to content dir.
        fs::copy(path, &target_path)
            .with_context(|| format!("while copying file `{path:?}` from source `{source}`"))?;

        // Apply patch if one exists
        self.apply_patch(&target_path)
            .with_context(|| format!("while patching `{path:?}` from source `{source}`"))?;

        self.add_to_filelist(rel, Some(&target_path))
            .with_context(|| format!("while adding file `{path:?}` from source `{source}`"))?;

        Ok(())
    }
}

// Public methods
impl FilePicker {
    /// Create a new file picker working in build_dir
    pub fn new(bundle_config: &BundleConfig, build_dir: PathBuf) -> Result<Self> {
        if !build_dir.is_dir() {
            bail!("build_dir is not a directory!")
        }

        if build_dir.read_dir()?.next().is_some() {
            bail!("build_dir is not empty!")
        }

        Ok(FilePicker {
            build_dir,

            filelist: HashMap::new(),
            diffs: HashMap::new(),

            search: bundle_config
                .search_order
                .iter()
                .map(|x| x.trim())
                .filter(|x| !(x.is_empty() || x.starts_with('#')))
                .map(Self::expand_search_line)
                .collect::<Result<Vec<Vec<String>>>>()?
                .into_iter()
                .flatten()
                .collect(),

            ignore_patterns: bundle_config
                .ignore
                .iter()
                .map(|x| String::from(x.trim()))
                .filter(|x| !(x.is_empty() || x.starts_with('#')))
                .map(|x| Regex::new(&format!("^{x}$")))
                .collect::<Result<Vec<Regex>, regex::Error>>()?,

            stats: PickStatistics::default(),
        })
    }

    /// Load all `diff` files in a directory into this picker's patch list.
    pub fn load_diffs_from(&mut self, path: &Path) -> Result<()> {
        for entry in WalkDir::new(&path) {
            // Only iterate files
            let entry = entry?;
            if !entry.file_type().is_file() {
                continue;
            }
            let entry = entry.into_path();

            // Only include files with a `.diff extension`
            if entry.extension().map(|x| x != "diff").unwrap_or(true) {
                continue;
            }

            // Read first line of diff to get target path
            let diff_file = fs::read_to_string(&entry).unwrap();
            let (target, _) = diff_file.split_once('\n').unwrap();

            trace!(tectonic_log_source = "select", "adding diff {entry:?}");

            for t in Self::expand_search_line(target)?
                .into_iter()
                .map(PathBuf::from)
            {
                if self.diffs.contains_key(&t) {
                    warn!(
                        tectonic_log_source = "select",
                        "the target of diff {entry:?} conflicts with another ignoring"
                    );
                    continue;
                }

                self.diffs.insert(t, entry.clone());
                self.stats.patch_found += 1;
            }
        }

        Ok(())
    }

    /// Add a directory of files to this bundle under `source_name`,
    /// applying patches and checking for replacements.
    pub fn add_tree(&mut self, source: &str, path: &Path) -> Result<()> {
        let mut added = 0usize;

        for entry in WalkDir::new(path) {
            // Only iterate files
            let entry = entry?;
            if !entry.file_type().is_file() {
                continue;
            }
            let entry = entry.into_path();

            // Skip ignored files
            if self.ignore_file(source, entry.strip_prefix(path).unwrap().to_str().unwrap()) {
                debug!(
                    tectonic_log_source = "select",
                    "skipping file {entry:?} from source `{source}` because of ignore patterns"
                );
                self.stats.ignored += 1;
                continue;
            }

            if self.filelist.len() % 1937 == 0 {
                info!(
                    tectonic_log_source = "select",
                    "selecting files ({source}, {})",
                    self.filelist.len()
                );
            }

            trace!(
                tectonic_log_source = "select",
                "adding file {entry:?} from source `{source}`"
            );

            self.add_file(
                &entry,
                source,
                entry.strip_prefix(path).unwrap().to_str().unwrap(),
            )?;
            added += 1;
        }

        self.stats.added.insert(source.to_owned(), added);

        Ok(())
    }

    pub fn finish(&mut self, save_debug_files: bool) -> Result<()> {
        info!(tectonic_log_source = "select", "writing auxillary files...");

        // Save search specification
        {
            let path = self.build_dir.join("content/SEARCH");

            let mut file = File::create(&path)?;
            for s in &self.search {
                writeln!(file, "{s}")?;
            }

            self.add_to_filelist(PathBuf::from("SEARCH"), Some(&path))?;
        }

        // Add auxillary files to the file list.
        {
            // These aren't hashed, but must be listed anyway.
            // The hash is generated from the filelist, so we must add these before hashing.
            self.add_to_filelist(PathBuf::from("SHA256SUM"), None)?;
            self.add_to_filelist(PathBuf::from("FILELIST"), None)?;

            let mut filelist_vec = Vec::from_iter(self.filelist.values());
            filelist_vec.sort_by(|a, b| a.path.cmp(&b.path));

            let filelist_path = self.build_dir.join("content/FILELIST");

            // Save FILELIST.
            let mut file = File::create(&filelist_path)?;
            for entry in filelist_vec {
                writeln!(file, "{entry}")?;
            }

            // Compute and save hash
            let mut file = File::create(self.build_dir.join("content/SHA256SUM"))?;

            let mut hasher = Sha256::new();
            let _ = std::io::copy(&mut fs::File::open(&filelist_path)?, &mut hasher)?;
            let hash = hasher
                .finalize()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<Vec<_>>()
                .concat();

            writeln!(file, "{hash}")?;
        }

        if save_debug_files {
            // Generate search-report
            {
                let mut file = File::create(self.build_dir.join("search-report"))?;
                for entry in WalkDir::new(&self.build_dir.join("content")) {
                    let entry = entry?;
                    if !entry.file_type().is_dir() {
                        continue;
                    }
                    let entry = entry
                        .into_path()
                        .strip_prefix(&self.build_dir.join("content"))
                        .unwrap()
                        .to_owned();
                    let entry = PathBuf::from("/").join(entry);

                    // Will this directory be searched?
                    let mut is_searched = false;
                    for rule in &self.search {
                        if rule.ends_with("//") {
                            // Match start of patent path
                            // (cutting off the last slash from)
                            if entry.starts_with(&rule[0..rule.len() - 1]) {
                                is_searched = true;
                                break;
                            }
                        } else {
                            // Match full parent path
                            if entry.to_str().unwrap() == rule {
                                is_searched = true;
                                break;
                            }
                        }
                    }

                    if !is_searched {
                        let s = entry.to_str().unwrap();
                        let t = s.matches('/').count();
                        writeln!(file, "{}{s}", "\t".repeat(t - 1))?;
                    }
                }
            }
        }
        Ok(())
    }
}
