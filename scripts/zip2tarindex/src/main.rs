// src/main.rs -- the zip2tarindex conversion helper program
// Copyright 2017-2020 The Tectonic Project
// Licensed under the MIT License.

use clap::{App, Arg};
use std::fs::File;
use std::io::{stderr, Cursor, Error, ErrorKind, Read, Write};
use std::os::unix::ffi::OsStringExt;
use std::{fmt, path, process};

// Here is stuff from tar-rs's lib.rs:

pub use entry_type::EntryType;
pub use header::GnuExtSparseHeader;
pub use header::{GnuHeader, GnuSparseHeader, Header, OldHeader, UstarHeader};
//pub use entry::Entry;
//pub use archive::{Archive, Entries};
pub use builder::HackedBuilder;
pub use pax::{PaxExtension, PaxExtensions};
use walkdir::WalkDir;


//mod archive;
mod builder;
//mod entry;
mod entry_type;
mod error;
mod header;
mod pax;

fn other(msg: &str) -> Error {
    Error::new(ErrorKind::Other, msg)
}

// End tar-rs copy-paste.

fn die(args: fmt::Arguments) -> ! {
    writeln!(&mut stderr(), "error: {}", args).expect("write to stderr failed");
    process::exit(1)
}

fn main() {
    let matches = App::new("zip2tarindex")
        .version("0.1")
        .about("Convert a Zip file to a tar file with an index.")
        .arg(
            Arg::with_name("SOURCEDIR")
                .help("A directory containing bundle contents.")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("TARPATH")
                .help("The name of the output tar file to create.")
                .required(true)
                .index(2),
        )
        .get_matches();

    let sourcepath = matches.value_of("SOURCEDIR").unwrap();
    let tarpath = matches.value_of("TARPATH").unwrap();

    // Open files.

    let mut tarfile = match File::create(tarpath) {
        Ok(f) => f,
        Err(e) => die(format_args!("failed to create \"{}\": {}", tarpath, e)),
    };

    let mut indexpath = path::PathBuf::from(tarpath);
    let mut tar_fn = indexpath.file_name().unwrap().to_os_string();
    tar_fn.push(".index.gz");
    indexpath.set_file_name(&tar_fn);
    let indexfile = match File::create(&indexpath) {
        Ok(f) => f,
        Err(e) => die(format_args!(
            "failed to create \"{}\": {}",
            indexpath.display(),
            e
        )),
    };

    // Stack up our I/O processing chain.

    let mut gzindex = flate2::GzBuilder::new()
        .filename(tar_fn.into_vec())
        .write(indexfile, flate2::Compression::default());

    let mut tar = HackedBuilder::new(&mut tarfile, &mut gzindex);

    // Ready to go.

    let mut header = Header::new_gnu();

    for entry in WalkDir::new(sourcepath).into_iter().filter_map(|e| e.ok()) {
        let p = entry.path();
        if p.is_dir() { continue; }

        let n = match p.file_name() {
            Some(a) => a.to_str().unwrap(),
            None => die(format_args!("couldn't get file name of \"{:?}\"", p))
        };

        let mut file = match File::open(p) {
            Ok(a) => a,
            Err(e) => die(format_args!("couldn't open file \"{}\": {}", n, e))
        };

        let size = match file.metadata() {
            Ok(a) => a.len(),
            Err(e) => die(format_args!("couldn't get size of \"{}\": {}", n, e))
        };

        let mut buf = Vec::with_capacity(size as usize);
        if let Err(e) = file.read_to_end(&mut buf) {
            die(format_args!("failure reading \"{}\": {}", n, e));
        }

        if let Err(e) = header.set_path(n) {
            die(format_args!("failure encoding filename \"{}\": {}", n, e));
        }

        header.set_size(size);
        header.set_cksum();

        if let Err(e) = tar.append(&header, Cursor::new(buf)) {
            die(format_args!("failure appending \"{}\" to tar: {}", n, e));
        }
    }

    if let Err(e) = tar.into_inner() {
        die(format_args!("error finishing tar file: {}", e));
    }
}
