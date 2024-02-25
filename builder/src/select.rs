use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fmt::Display,
    fs::{self, File},
    io::{stdout, Write},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use regex::Regex;
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

#[derive(Default)]
struct PickStatistics {
    extra: usize,
    extra_conflict: usize,
    added: HashMap<String, usize>,
    ignored: usize,
    replaced: usize,
    patch_applied: usize,
}

struct FileListEntry {
    // Path relative to content
    // (does not start with a slash)
    path: PathBuf,

    // Hash string or "nohash"
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
    include: PathBuf,
    output: PathBuf,
    content: PathBuf,

    filelist: Vec<FileListEntry>,
    extra_basenames: HashSet<String>,
    diffs: HashMap<PathBuf, PathBuf>,
    ignore_patterns: Vec<Regex>,
    search: Vec<String>,

    stats: PickStatistics,

    // Used to prettyprint.
    last_print_len: usize,
}

impl FilePicker {
    // Transform a search order file with shortcuts
    // (bash-like brace expansion, like `/a/b/{tex,latex}/c`)
    // into a plain list of strings.
    fn expand_search_line(s: &str) -> Result<Vec<String>, Box<dyn Error>> {
        if !(s.contains('{') || s.contains('}')) {
            return Ok(vec![s.to_owned()]);
        }

        let first = s.find('{').ok_or("Bad search path format")?;
        let last = s.find('}').ok_or("Bad search path format")?;

        let head = &s[..first];
        let mid = &s[first + 1..last];

        if mid.contains('{') || mid.contains('}') {
            // Mismatched or nested braces
            return Err("Bad search path format".into());
        }

        // We find the first brace, so only tail may have other expansions.
        let tail = Self::expand_search_line(&s[last + 1..s.len()])?;

        if mid.is_empty() {
            return Err("Bad search path format".into());
        }

        let mut output: Vec<String> = Vec::new();
        for m in mid.split(',') {
            for t in &tail {
                if m.is_empty() {
                    return Err("Bad search path format".into());
                }
                output.push(format!("{}{}{}", head, m, t));
            }
        }

        Ok(output)
    }

    fn consider_file(&self, source: &str, file_rel_path: &str) -> bool {
        let f = format!("/{source}/{file_rel_path}");
        for pattern in &self.ignore_patterns {
            if pattern.is_match(&f) {
                return false;
            }
        }

        true
    }

    fn apply_patch(&mut self, path: &Path) -> Result<bool, Box<dyn Error>> {
        // path is absolute, but self.diffs is indexed by
        // paths relative to content dir.
        let path_rel = path.strip_prefix(&self.content)?;

        // Is this file patched?
        if !self.diffs.contains_key(path_rel) {
            return Ok(false);
        }

        // Debug print
        let s = format!(
            "Patching {}",
            path.file_name()
                .ok_or("Couldn't get file name".to_string())?
                .to_str()
                .ok_or("Couldn't get file name as str".to_string())?
        );
        if s.len() < self.last_print_len {
            println!("\r{s}{}", " ".repeat(self.last_print_len - s.len()));
        } else {
            println!("\r{s}");
        }

        self.stats.patch_applied += 1;

        // Discard first line of diff
        let diff_file = fs::read_to_string(&self.diffs[path_rel]).unwrap();
        let (_, diff) = diff_file.split_once('\n').unwrap();

        let mut child = Command::new("patch")
            .arg("--quiet")
            .arg("--no-backup")
            .arg(path)
            .stdin(Stdio::piped())
            .spawn()?;

        let mut stdin = child.stdin.take().unwrap();
        stdin.write_all(diff.as_bytes())?;
        drop(stdin);
        child.wait()?;

        Ok(true)
    }

    // Add a file into the file list.
    // path: path to file, relative to content
    // file: path to file for hash, None if no hash is available.
    fn add_to_filelist(
        &mut self,
        path: PathBuf,
        file: Option<&Path>,
    ) -> Result<(), Box<dyn Error>> {
        self.filelist.push(FileListEntry {
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
        });

        Ok(())
    }

    fn add_file(
        &mut self,
        path: &Path,
        source: &str,
        file_rel_path: &str,
    ) -> Result<(), Box<dyn Error>> {
        let target_path = self.content.to_path_buf().join(source).join(file_rel_path);

        // Path to this file, relative to content dir
        let rel = target_path
            .strip_prefix(&self.content)
            .unwrap()
            .to_path_buf();

        fs::create_dir_all(
            target_path
                .parent()
                .ok_or("Couldn't get parent".to_string())?,
        )?;

        // Copy to content dir.
        // Extracted dir is read-only, we need to chmod to patch.
        fs::copy(path, &target_path)?;
        fs::set_permissions(&target_path, fs::Permissions::from_mode(0o664))?;

        // Apply patch if one exists
        self.apply_patch(&target_path)?;

        self.add_to_filelist(rel, Some(&target_path))?;

        Ok(())
    }
}

// Public methods
impl FilePicker {
    pub fn new(
        bundle_dir: &Path,
        build_dir: &Path,
        bundle_name: &str,
    ) -> Result<Self, Box<dyn Error>> {
        Ok(FilePicker {
            // Paths
            include: bundle_dir.join("include"),
            content: build_dir.join("output").join(bundle_name).join("content"),
            output: build_dir.join("output").join(bundle_name),

            // Various arrays
            filelist: Vec::new(),
            extra_basenames: HashSet::new(),
            diffs: HashMap::new(),

            search: fs::read_to_string(bundle_dir.join("search-order"))
                .unwrap_or("".to_string())
                .lines()
                .map(|x| x.trim())
                .filter(|x| !(x.is_empty() || x.starts_with('#')))
                .map(Self::expand_search_line)
                .collect::<Result<Vec<Vec<String>>, Box<dyn Error>>>()?
                .into_iter()
                .flatten()
                .collect(),

            ignore_patterns: fs::read_to_string(bundle_dir.join("ignore"))
                .unwrap_or("".to_string())
                .lines()
                .map(|x| String::from(x.trim()))
                .filter(|x| !(x.is_empty() || x.starts_with('#')))
                .map(|x| Regex::new(&format!("^{x}$")))
                .collect::<Result<Vec<Regex>, regex::Error>>()?,

            stats: PickStatistics::default(),
            last_print_len: 0,
        })
    }

    pub fn add_extra(&mut self) -> Result<(), Box<dyn Error>> {
        // Only iterate files
        for entry in WalkDir::new(&self.include) {
            let entry = entry?;
            if !entry.file_type().is_file() {
                continue;
            }
            let entry = entry.into_path();

            let name = entry
                .file_name()
                .ok_or("Couldn't get file name".to_string())?
                .to_str()
                .ok_or("Couldn't get file name as str".to_string())?;

            if entry.extension().map(|x| x == "diff").unwrap_or(false) {
                // Read first line of diff to get target path
                let diff_file = fs::read_to_string(&entry).unwrap();
                let (target, _) = diff_file.split_once('\n').unwrap();

                for t in Self::expand_search_line(target)?
                    .into_iter()
                    .map(PathBuf::from)
                {
                    if self.diffs.contains_key(&t) {
                        println!("Warning: included diff {name} has target conflict, ignoring");
                        continue;
                    }

                    self.diffs.insert(t, entry.clone());
                }

                continue;
            }

            if self.extra_basenames.contains(name) {
                self.stats.extra_conflict += 1;
                println!("Warning: included file {name} has conflicts, ignoring");
                continue;
            }

            self.add_file(
                &entry,
                "include",
                entry.strip_prefix(&self.include).unwrap().to_str().unwrap(),
            )?;

            self.stats.extra += 1;
            self.extra_basenames.insert(name.to_owned());
        }

        Ok(())
    }

    pub fn add_tree(&mut self, source_name: &str, path: &Path) -> Result<(), Box<dyn Error>> {
        let mut added = 0usize;

        // Only iterate files
        for entry in WalkDir::new(path) {
            let entry = entry?;
            if !entry.file_type().is_file() {
                continue;
            }
            let entry = entry.into_path();

            if added % 193 == 0 {
                let s = format!("\r[{}] Selecting files... {}", source_name, added);
                self.last_print_len = s.len();
                print!("{}", s);
                stdout().flush()?;
            }

            if !self.consider_file(
                source_name,
                entry.strip_prefix(path).unwrap().to_str().unwrap(),
            ) {
                self.stats.ignored += 1;
                continue;
            }

            let name = entry.file_name().unwrap().to_str().unwrap();

            if self.extra_basenames.contains(name) {
                self.stats.replaced += 1;
                continue;
            }

            self.add_file(
                &entry,
                source_name,
                entry.strip_prefix(path).unwrap().to_str().unwrap(),
            )?;
            added += 1;
        }

        self.stats.added.insert(source_name.to_owned(), added);
        println!("\r[{source_name}] Selecting files... Done!       ");
        println!();

        Ok(())
    }

    pub fn add_search(&mut self) -> Result<(), Box<dyn Error>> {
        let path = self.content.join("SEARCH");

        let mut file = File::create(&path)?;
        for s in &self.search {
            writeln!(file, "{s}")?;
        }

        self.add_to_filelist(PathBuf::from("SEARCH"), Some(&path))?;

        Ok(())
    }

    pub fn add_meta_files(&mut self) -> Result<(), Box<dyn Error>> {
        // Add auxillary files to the file list.
        // These aren't hashed, but they must be listed anyway
        // Our hash is generated from the filelist, so we need to add these before doing that.
        self.add_to_filelist(PathBuf::from("SHA256SUM"), None)?;
        self.add_to_filelist(PathBuf::from("FILELIST"), None)?;

        let mut filelist_vec = Vec::from_iter(self.filelist.iter());
        filelist_vec.sort_by(|a, b| a.path.cmp(&b.path));

        let filelist_path = self.content.join("FILELIST");

        // Save FILELIST.
        let mut file = File::create(&filelist_path)?;
        for entry in filelist_vec {
            writeln!(file, "{entry}")?;
        }

        // Compute and save hash
        let mut file = File::create(self.content.join("SHA256SUM"))?;

        let mut hasher = Sha256::new();
        let _ = std::io::copy(&mut fs::File::open(&filelist_path)?, &mut hasher)?;
        let hash = hasher
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<Vec<_>>()
            .concat();

        writeln!(file, "{hash}")?;

        Ok(())
    }

    pub fn generate_debug_files(&self) -> Result<(), Box<dyn Error>> {
        // Generate search-report
        let mut file = File::create(self.output.join("search-report"))?;
        for entry in WalkDir::new(&self.content) {
            let entry = entry?;
            if !entry.file_type().is_dir() {
                continue;
            }
            let entry = entry
                .into_path()
                .strip_prefix(&self.content)
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

        Ok(())
    }

    pub fn show_summary(&self) {
        println!(
            concat!(
                "\n",
                "=============== Summary ===============\n",
                "    extra file conflicts: {}\n",
                "    files ignored:        {}\n",
                "    files replaced:       {}\n",
                "    diffs applied/found:  {}/{}\n",
                "    =============================\n",
                "    extra files:          {}",
            ),
            self.stats.extra_conflict,
            self.stats.ignored,
            self.stats.replaced,
            self.stats.patch_applied,
            self.diffs.len(),
            self.stats.extra,
        );

        let mut sum = self.stats.extra;
        for (source, count) in &self.stats.added {
            let s = format!("{source} files: ");
            println!("    {s}{}{count}", " ".repeat(22 - s.len()));
            sum += count;
        }
        println!("    total files:          {sum}");
        println!();

        if self.diffs.len() > self.stats.patch_applied {
            println!("Warning: not all diffs were applied")
        }

        if self.diffs.len() < self.stats.patch_applied {
            println!("Warning: some diffs were applied multiple times")
        }

        println!("=======================================");
    }
}
