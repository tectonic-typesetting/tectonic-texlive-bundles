use std::{
    collections::{HashMap, HashSet},
    env,
    fs::{self, File},
    io::{stdout, Write},
    path::{Path, PathBuf},
    process::Command
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

struct FilePicker {
    include: PathBuf,
    output: PathBuf,
    content: PathBuf,

    index: HashMap<String, Vec<PathBuf>>,
    item_shas: HashMap<PathBuf, String>,
    extra_basenames: HashSet<String>,
    diffs: HashMap<String, PathBuf>,
    ignore_patterns: Vec<Regex>,
    search: Vec<String>,

    stats: PickStatistics,

    // Used to prettyprint.
    last_print_len: usize,
}

macro_rules! add_to_index {
    ($index:expr, $name:literal) => {
        $index.insert($name.to_string(), vec![PathBuf::from($name)]);
    };
}

impl FilePicker {
    fn new(bundle_dir: &Path, build_dir: &Path, bundle_name: &str) -> Self {
        FilePicker {
            // Paths
            include: bundle_dir.join("include"),
            content: build_dir.join("output").join(&bundle_name).join("content"),
            output: build_dir.join("output").join(&bundle_name),

            // Various arrays
            index: HashMap::new(),
            item_shas: HashMap::new(),
            extra_basenames: HashSet::new(),
            diffs: HashMap::new(),

            ignore_patterns: fs::read_to_string(bundle_dir.join("ignore"))
                .unwrap_or("".to_string())
                .split("\n")
                .map(|x| String::from(x.trim()))
                .filter(|x| (x.len() != 0) && (!x.starts_with('#')))
                .map(|x| Regex::new(&format!("^{x}$")).unwrap())
                .collect(),

            search: fs::read_to_string(bundle_dir.join("search-order"))
                .unwrap_or("".to_string())
                .split("\n")
                .map(|x| String::from(x.trim()))
                .filter(|x| (x.len() != 0) && (!x.starts_with('#')))
                .collect(),

            stats: PickStatistics::default(),
            last_print_len: 0,
        }
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

    fn has_patch(&self, path: &Path) -> bool {
        let name = path.file_name().unwrap().to_str().unwrap();
        return self.diffs.contains_key(name);
    }

    fn apply_patch(&mut self, path: &Path) -> bool {
        if !self.has_patch(path) {
            return false;
        }
        let name = path.file_name().unwrap().to_str().unwrap();

        let s = format!("Patching {name}");
        if s.len() < self.last_print_len {
            println!("\r{s}{}", " ".repeat(self.last_print_len - s.len()));
        } else {
            println!("\r{s}");
        }

        self.stats.patch_applied += 1;

        Command::new("patch")
            .arg("--quiet")
            .arg("--no-backup")
            .arg(path)
            .arg(&self.diffs[name])
            .output()
            .expect("Patch failed");

        return true;
    }

    fn add_file(&mut self, path: &Path, source: &str, file_rel_path: &str) {
        let target_path = self.content.to_path_buf().join(source).join(file_rel_path);
        let name = path.file_name().unwrap().to_str().unwrap();

        // Add path to index
        let rel = target_path
            .strip_prefix(&self.content)
            .unwrap()
            .to_path_buf();
        let v = self.index.get_mut(name);
        if v.is_none() {
            self.index.insert(name.to_owned(), vec![rel.clone()]);
        } else {
            v.unwrap().push(rel.clone());
        }

        fs::create_dir_all(target_path.parent().unwrap()).expect("FS error");
        fs::copy(path, &target_path).expect("FS Error");

        // Apply patch if one exists
        self.apply_patch(&target_path);

        // Compute and save hash
        let digest = try_digest(target_path).unwrap();
        self.item_shas.insert(rel, digest);
    }

    fn add_extra(&mut self) {
        // Only iterate files
        for entry in WalkDir::new(&self.include) {
            let entry = entry.unwrap();
            if !entry.file_type().is_file() {
                continue;
            }
            let entry = entry.into_path();

            let name = entry
                .file_name()
                .expect("Couldn't get file name")
                .to_str()
                .unwrap();

            if entry.extension().map(|x| x == "diff").unwrap_or(false) {
                let n = &name[..name.len() - 5];
                if self.diffs.contains_key(n) {
                    println!("Warning: included diff {name} has conflicts, ignoring");
                    continue;
                }
                self.diffs.insert(n.to_owned(), entry);
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
            );
            self.stats.extra += 1;
            self.extra_basenames.insert(name.to_owned());
        }
    }

    fn add_tree(&mut self, source_name: &str, path: &Path) {
        let mut added = 0usize;

        // Only iterate files
        for entry in WalkDir::new(path) {
            let entry = entry.unwrap();
            if !entry.file_type().is_file() {
                continue;
            }
            let entry = entry.into_path();

            if added % 193 == 0 {
                let s = format!(
                    "\r[{}] Selecting files... {}",
                    source_name,
                    added
                );
                self.last_print_len = s.len();
                print!("{}", s);
                stdout().flush().unwrap();
            }

            if !self.consider_file(
                source_name,
                entry.strip_prefix(&path).unwrap().to_str().unwrap(),
            ) {
                self.stats.ignored += 1;
                continue;
            }

            let name = entry
                .file_name()
                .expect("Couldn't get file name")
                .to_str()
                .unwrap();

            if self.extra_basenames.contains(name) {
                self.stats.replaced += 1;
                continue;
            }

            self.add_file(
                &entry,
                source_name,
                entry.strip_prefix(&path).unwrap().to_str().unwrap(),
            );
            added += 1;
        }

        self.stats.added.insert(source_name.to_owned(), added);
        println!("\r[{source_name}] Selecting files... Done!       ");
        println!("");
    }

    fn add_search(&mut self) {
        let path = self.content.join("SEARCH");

        let mut file = File::create(&path).unwrap();
        for s in &self.search {
            writeln!(file, "{s}").unwrap();
        }

        // Add to index and hash search paths
        self.index.insert("SEARCH".to_string(), vec![path.clone()]);
        let digest = try_digest(&path).unwrap();
        self.item_shas.insert(path, digest);
    }

    fn add_meta_files(&mut self) {
        let mut index_vec = Vec::from_iter(self.index.iter());
        index_vec.sort_by(|a, b| a.0.cmp(b.0));

        // Add auxillary files to index.
        // These aren't hashed, but they need to be indexed.
        // Our hash is generated from the index, so we need to add these first.
        add_to_index!(self.index, "SHA256SUM");
        add_to_index!(self.index, "INDEX");

        // Sort index so hashes are reproducible.
        // Break ties with path.
        let mut index_vec = Vec::from_iter(self.index.iter());
        index_vec.sort_by(|a, b| match a.0.cmp(b.0) {
            std::cmp::Ordering::Equal => a.1.cmp(b.1),
            _ => a.0.cmp(b.0),
        });

        let index_path = self.content.join("INDEX");

        // Save index.
        let mut file = File::create(&index_path).unwrap();
        for (name, paths) in index_vec {
            let mut paths = paths.clone();
            paths.sort();
            for p in paths {
                match self.item_shas.get(&p) {
                    None => writeln!(file, "{name} {}", p.to_str().unwrap()).unwrap(),
                    Some(d) => writeln!(file, "{name} {} {d}", p.to_str().unwrap()).unwrap(),
                };
            }
        }

        // Compute and save hash
        let mut file = File::create(self.content.join("SHA256SUM")).unwrap();
        writeln!(file, "{}", try_digest(&index_path).unwrap()).unwrap();
    }

    fn generate_debug_files(&self) {
        // This is essentially a detailed version of SHA256SUM,
        // Good for finding file differences between bundles
        let mut file = File::create(self.output.join("file-hashes")).unwrap();
        for (path, hash) in &self.item_shas {
            writeln!(file, "{}\t{hash}", path.to_str().unwrap()).unwrap();
        }

        let mut file = File::create(self.output.join("search-report")).unwrap();
        for (_, paths) in &self.index {
            if !self.search_for_file(&paths) {
                for p in paths {
                    writeln!(file, "{}", p.to_str().unwrap()).unwrap();
                }
            }
        }
    }

    // Turn a name into a path
    fn search_for_file(&self, paths: &Vec<PathBuf>) -> bool {
        let name = paths[0].file_name().unwrap().to_str().unwrap();
        let paths: Vec<String> = paths.iter().map(|x| x.to_str().unwrap().into()).collect();

        for rule in &self.search {
            for path in &paths {
                if rule.ends_with("//") {
                    // Match start of patent path
                    // (cutting off the last slash from)
                    if path.starts_with(&rule[0..rule.len() - 1]) {
                        return true;
                    }
                } else {
                    // Match full parent path
                    if &path[0..path.len() - name.len()] == rule {
                        return true;
                    }
                }
            }
        }
        return false;
    }

    fn show_summary(&self) {
        println!(
            concat!(
                "\n",
                "============== Summary ==============\n",
                "    extra file conflicts: {}\n",
                "    files ignored:        {}\n",
                "    files replaced:       {}\n",
                "    diffs applied/found:  {}/{}\n",
                "    =================================\n",
                "    extra files added:    {}",
            ),
            self.stats.extra_conflict,
            self.stats.ignored,
            self.stats.replaced,
            self.stats.patch_applied,
            self.diffs.len(),
            self.stats.extra,
        );

        let mut sum = 0usize;
        for (source, count) in &self.stats.added {
            let s = format!("{source}: ");
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

        println!("=====================================");
    }
}



macro_rules! load_envvar {
    ($varname:ident, $type:ident) => {
        let $varname: $type = match env::var_os(stringify!($varname)) {
            Some(val) => $type::from(val.to_str().unwrap()),
            None => panic!(concat!("Expected envvar ", stringify!($varname))),
        };
    };
}

fn main() {

    // Read environment variables
    load_envvar!(bundle_dir, PathBuf);
    load_envvar!(build_dir, PathBuf);
    load_envvar!(bundle_texlive_name, String);
    load_envvar!(bundle_name, String);

    let mut picker = FilePicker::new(&bundle_dir, &build_dir, &bundle_name);

    picker.add_extra();

    picker.add_tree(
        "texlive",
        &build_dir.join("texlive").join(&bundle_texlive_name),
    );

    println!("Preparing auxillary files...");
    picker.add_search();
    picker.add_meta_files();
    picker.generate_debug_files();
    picker.show_summary();
}
