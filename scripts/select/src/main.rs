use std::{
    collections::{HashMap, HashSet},
    env,
    error::Error,
    fs::{self, File},
    io::{stdout, Write},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use regex::Regex;
use sha256::try_digest;
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

struct IndexEntry {
    // Path relative to content
    // (does not start with a slash)
    path: PathBuf,

    // Hash string or "nohash"
    hash: Option<String>,
}

impl ToString for IndexEntry {
    // Returns this indexentry as a line in the INDEX file.
    fn to_string(&self) -> String {
        return format!(
            "/{} {}",
            self.path.to_str().unwrap(),
            match &self.hash {
                Some(s) => &s,
                None => "nohash",
            }
        );
    }
}

struct FilePicker {
    include: PathBuf,
    output: PathBuf,
    content: PathBuf,

    index: Vec<IndexEntry>,
    extra_basenames: HashSet<String>,
    diffs: HashMap<PathBuf, PathBuf>,
    ignore_patterns: Vec<Regex>,
    search: Vec<String>,

    stats: PickStatistics,

    // Used to prettyprint.
    last_print_len: usize,
}

// Insert a file into a FilePicker index, without a hash.
// Used for generated files.
macro_rules! add_to_index {
    ($picker:expr, $path:expr) => {
        $picker.index.push(IndexEntry {
            path: PathBuf::from($path),
            hash: None,
        })
    };

    ($picker:expr, $path:expr, $hash:expr) => {
        $picker.index.push(IndexEntry {
            path: PathBuf::from($path),
            hash: Some($hash),
        })
    };
}

impl FilePicker {
    // Transform a search order file with shortcuts
    // (bash-like brace expansion, like `/a/b/{tex,latex}/c`)
    // into a plain list of strings.
    fn expand_search_line(s: &str) -> Result<Vec<String>, Box<dyn Error>> {
        if !(s.contains('{') || s.contains('}')) {
            return Ok(vec![s.to_owned()]);
        }

        let first = s.find("{").ok_or("Bad search path format")?;
        let last = s.find("}").ok_or("Bad search path format")?;

        let head = &s[..first];
        let mid = &s[first + 1..last];

        if mid.contains('{') || mid.contains('}') {
            // Mismatched or nested braces
            return Err("Bad search path format".into());
        }

        // We find the first brace, so only tail may have other expansions.
        let tail = Self::expand_search_line(&s[last + 1..s.len()])?;

        if mid.len() == 0 {
            return Err("Bad search path format".into());
        }

        let mut output: Vec<String> = Vec::new();
        for m in mid.split(",") {
            for t in &tail {
                if m.len() == 0 {
                    return Err("Bad search path format".into());
                }
                output.push(format!("{}{}{}", head, m, t));
            }
        }

        return Ok(output);
    }

    fn new(bundle_dir: &Path, build_dir: &Path, bundle_name: &str) -> Result<Self, Box<dyn Error>> {
        Ok(FilePicker {
            // Paths
            include: bundle_dir.join("include"),
            content: build_dir.join("output").join(&bundle_name).join("content"),
            output: build_dir.join("output").join(&bundle_name),

            // Various arrays
            index: Vec::new(),
            extra_basenames: HashSet::new(),
            diffs: HashMap::new(),

            search: fs::read_to_string(&bundle_dir.join("search-order"))
                .unwrap_or("".to_string())
                .split("\n")
                .map(|x| x.trim())
                .filter(|x| (x.len() != 0) && (!x.starts_with('#')))
                .map(|x| Self::expand_search_line(x))
                .collect::<Result<Vec<Vec<String>>, Box<dyn Error>>>()?
                .into_iter()
                .flatten()
                .collect(),

            ignore_patterns: fs::read_to_string(bundle_dir.join("ignore"))
                .unwrap_or("".to_string())
                .split("\n")
                .map(|x| String::from(x.trim()))
                .filter(|x| (x.len() != 0) && (!x.starts_with('#')))
                .map(|x| Regex::new(&format!("^{x}$")))
                .collect::<Result<Vec<Regex>, regex::Error>>()?,

            stats: PickStatistics::default(),
            last_print_len: 0,
        })
    }

    fn consider_file(&self, source: &str, file_rel_path: &str) -> bool {
        let f = format!("/{source}/{file_rel_path}");
        for pattern in &self.ignore_patterns {
            if pattern.is_match(&f) {
                return false;
            }
        }
        return true;
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

        return Ok(true);
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

        // Compute hash and add to index
        add_to_index!(self, rel, try_digest(target_path)?);

        return Ok(());
    }

    fn add_extra(&mut self) -> Result<(), Box<dyn Error>> {
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
                    .map(|x| PathBuf::from(x))
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

        return Ok(());
    }

    fn add_tree(&mut self, source_name: &str, path: &Path) -> Result<(), Box<dyn Error>> {
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
                entry.strip_prefix(&path).unwrap().to_str().unwrap(),
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
                entry.strip_prefix(&path).unwrap().to_str().unwrap(),
            )?;
            added += 1;
        }

        self.stats.added.insert(source_name.to_owned(), added);
        println!("\r[{source_name}] Selecting files... Done!       ");
        println!("");

        return Ok(());
    }

    fn add_search(&mut self) -> Result<(), Box<dyn Error>> {
        let path = self.content.join("SEARCH");

        let mut file = File::create(&path)?;
        for s in &self.search {
            writeln!(file, "{s}")?;
        }

        // Add to index and hash search paths
        add_to_index!(self, "SEARCH", try_digest(&path)?);

        return Ok(());
    }

    fn add_meta_files(&mut self) -> Result<(), Box<dyn Error>> {
        // Add auxillary files to index.
        // These aren't hashed, but they need to be indexed.
        // Our hash is generated from the index, so we need to add these first.
        add_to_index!(self, "SHA256SUM");
        add_to_index!(self, "INDEX");

        let mut index_vec = Vec::from_iter(self.index.iter());
        index_vec.sort_by(|a, b| a.path.cmp(&b.path));

        let index_path = self.content.join("INDEX");

        // Save index.
        let mut file = File::create(&index_path)?;
        for index_entry in index_vec {
            writeln!(file, "{}", index_entry.to_string())?;
        }

        // Compute and save hash
        let mut file = File::create(self.content.join("SHA256SUM"))?;
        writeln!(file, "{}", try_digest(&index_path)?)?;

        return Ok(());
    }

    fn generate_debug_files(&self) -> Result<(), Box<dyn Error>> {
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
                let t = s.matches("/").count();
                writeln!(file, "{}{s}", "\t".repeat(t - 1))?;
            }
        }

        return Ok(());
    }

    fn show_summary(&self) {
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
        println!("");

        if self.diffs.len() > self.stats.patch_applied {
            println!("Warning: not all diffs were applied")
        }

        if self.diffs.len() < self.stats.patch_applied {
            println!("Warning: some diffs were applied multiple times")
        }

        println!("=======================================");
    }
}

macro_rules! load_envvar {
    ($varname:ident, $type:ident) => {
        let $varname: $type = match env::var_os(stringify!($varname)) {
            Some(val) => Ok($type::from(val.to_str().unwrap())),
            None => Err(concat!("Expected envvar ", stringify!($varname))),
        }?;
    };
}

fn main() -> Result<(), Box<dyn Error>> {
    // Read environment variables
    load_envvar!(bundle_dir, PathBuf);
    load_envvar!(build_dir, PathBuf);
    load_envvar!(bundle_texlive_name, String);
    load_envvar!(bundle_name, String);

    let mut picker = FilePicker::new(&bundle_dir, &build_dir, &bundle_name)?;

    picker.add_extra()?;

    picker.add_tree(
        "texlive",
        &build_dir.join("texlive").join(&bundle_texlive_name),
    )?;

    println!("Preparing auxillary files...");
    picker.add_search()?;
    picker.add_meta_files()?;
    picker.generate_debug_files()?;
    picker.show_summary();

    return Ok(());
}
