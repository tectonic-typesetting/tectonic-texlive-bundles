# Tectonic Bundle Builder

This repository contains scripts for building bundles for
[Tectonic](https://tectonic-typesetting.github.io), each of which is a complete TeX distribution.

**You do not need this repository to build Tectonic.** \
You only need these scripts if you want to make your own bundles of TeX files.

**Warning:** The `./tests` do not work yet, they need to be reworked for the new bundle spec!





## Prerequisites

To use these tools, you will need:

- Bash, `pv`, GNU `patch` and `diff`
- A [TeXlive tarball](https://tug.org/texlive/acquire-tar.html). Different bundles need different TeXlive versions.
- A Rust toolchain (`cargo`).




## Bundles:
Each directory in `./bundles` is a bundle specification, which contains everything we need to reproducibly build a bundle.\
See [`./bundles/README.md`](./bundles/README.md) for details.

The following bundles are available:
 - `texlive/2022.0r0`: directly copied from the bundle in `master`. \
 Uses `texlive-2022.0r0` and is probably broken.

 - `texlive2023-nopatch`: based on `texlive2023-20230313`.






## Build Process:
Before building any bundles, acquire a [TeXlive tarball](https://tug.org/texlive/acquire-tar.html) with a version that matches the bundle you want to build. These are usually distributed as compressed tarballs, which you'll have to manually decompress. **`build.sh` expects an uncompressed `.tar` file!** It checks the hash of this file, and will refuse to work if that hash doesn't match.

The tarball you end up with should have a name of the form `texlive-YYYYMMDD-texmf.tar`. **Do not rename this file!**



To build a bundle, run the following jobs. These **must** be run in order!

 - `./build.sh <tarball> extract`: extracts texlive into `./build/texlive/<version>`\
  This also generates `TEXLIVE-SHA256SUM` in the texlive version directory.

 - `./build.sh <bundle> content`: assemble all files into a bundle at `./build/output/<bundle>content`.\
  This will delete all bundles in `output/<bundle>/`, move them elsewhere if you still need them.

Once `./build/output/content` has been created, run any of the following commands to package the bundle:

 - `./build.sh <bundle> ttbv1`: create a ttb (version 1) bundle from the content directory.\
  TTB bundles may be used locally or hosted on the web.


## Output Files


**`./build.sh <bundle> content` produces the following:**
 - `./build/output/<bundle>/content`: contains all bundle files. It is organized by source: files from the bundle's `include` dir will be under `./include`, texlive files will be under `./texlive`, and so on. See `main.rs` of `scripts/select`.
 This directory also contains some metadata:

   - `content/INDEX`: each line of this file is `<path> <hash>`, sorted by file name.\
   Files with identical names are included.\
   Files not in any search path are also included.\
   `<hash>` is either a hex sha256 of that file's contents, or `nohash` for a few special files.

   - `content/SHA256SUM`: The sha256sum of `content/INDEX`. This string uniquely defines this bundle. \

   - `content/SEARCH`: File search order for this bundle. See bundle spec documentation.

 - `search-report`: debug file. Lists all directories that will not be searched by the rules in `search-order`.\
  The entries in this file are non-recursive: If `search-report` contains a line with `/texlive`, this means that direct children of `/texlive` (like `/texlive/file.tex`) will not be found, but files in *subdirectories* (like `/texlive/tex/file.tex`) may be.


**`./build.sh <bundle> ttbv1` produces the following:**
 - `<bundle>.ttb`: the bundle. Note that the ttb version is *not* included in the extension.