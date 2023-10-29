# Tectonic Bundle Builder

This repository contains scripts for building bundles for
[Tectonic](https://tectonic-typesetting.github.io), each of which is a complete TeX distribution.

**You do not need this repository to build Tectonic.** \
You only need these scripts if you want to make your own bundles of TeX files.

**Warning:** The `./tests` do not work yet, they still need to be reworked for the new bundle spec!








## Prerequisites

To use these tools, you will need:

- Bash
- Python 3.11 & Python standard packages
- An installation of [Docker](https://www.docker.com/).
- A [TeXlive iso](https://tug.org/texlive/acquire-iso.html). Different bundles need different TeXlive versions.
- A Rust toolchain if you want to create “indexed tar” bundles. You don’t
  need Rust if you want to create a bundle and test it locally.








## Bundles:
Each directory in `./bundles` is a *bundle specification*. Each contains everything we need to reproducibly build a bundle. See [`./bundles/README.md`](./bundles/README.md) for details.

The following bundles are available:
 - `texlive/2022.0r0`: all of TeXlive, plus a few patches. Directly copied from `master`. \
 Uses `texlive-2022.0r0`.

 - `texlive2023-nopatch`: texlive2023 with no patches. \
 Uses `texlive-2023.0r0`.









## Build Process:
Before building any bundles, acquire a [TeXlive iso](https://tug.org/texlive/acquire-iso.html) with a version that matches the bundle you want to build. `build.sh` checks the hash of this file, unless you run `forceinstall` instead of `install`.

To build a bundle, run `./build <bundle dir> all <iso path>`. This executes the following jobs in order:
 - `./build container`: builds the docker container from `./docker`
 - `./build <bundle> install <iso>`: installs TeXLive to `./build/install/`
 - `./build <bundle> zip <iso>`: creates a zip bundle. Local zip bundles may be used in place of web bundles (see tectonic docs).
 - `./build <bundle> itar`: converts that zip to an indexed tar bundle. This will NOT work if a zip bundle doesn't exist.
 itar bundles may not be used locally, they are only used as web bundles. If you want to host your own, you'll need to put `bundle.tar` and `<bundle>.tar.sha256sum` under the same url.

Each of the steps above requires the previous steps. You may execute them manually, although
there's no reason to do this unless something breaks.


### Build Notes:
 - The `install` job could take a while. `tail -f` its log file to watch progress.
 - `install` will fail if your iso hash does not match the hash of the iso the bundle was designed for.\
 This may be overriden by replacing `./build.sh <bundle> install <iso>` with `./build.sh <bundle> forceinstall <iso>`.







## Output Files

**`./build.sh <bundle> zip` produces the following:**
 - `$bundle.zip`: the main zip bundle
 - `clash-report.txt`: debug file. did any files have the same name? (if any)
 - `$bundle.listing.txt`: a sorted list of all files in the bundle
 - `$bundle.texlive-sha256sum`: a hash of the TeXlive image used to build this bundle. This file is also in the zip as `TEXLIVE-SHA256SUM`.
 - `file-hashes`: debug file. Indexes the contents of the bundle. Used to find which files differ between two builds.
 - `$bundle.sha256sum`: a hash of the zip's contents. This file is also in the zip as `SHA256SUM`. \
 `file-hashes` and `$bundle.sha256sum` are generated in roughly the same way. The `file-hashes` files from two different
 bundles should match if and only if the two bundles have the same sha256sum.

**`./build.sh <bundle> itar` produces the following:**\
Note that both `$bundle.tar` and `$bundle.tar.index.gz` are required to host a web bundle.
 - `$bundle.tar`: the tar bundle
 - `$bundle.tar.index.gz`: the (compressed) tar index, with format `<file> <start> <len>`\
 This tells us that the first bit of `<file>` is at `<start>`, and the last is at `<start> + <len> - 1`.\
 You can extract a file from a local bundle using `dd if=file.tar ibs=1 skip=<start> count=<len>`\
 Or from a web bundle with `curl -r <start>-<start>+<len> https://url.tar`







## Reproducibility
The `SHA256HASH` stored in each bundle may change, even for identical builds! It depends on the contents of each file, and some files change every build. We do our best to prevent this.

**The following files contain a timestamp:**
 - `fmtutil.cnf`
 - `mf.base`
 - `updmap.cfg`

**The following files contain a UUID:** \
Most have *both* a UUID and a timestamp.
 - `357744afc7b3a35aafa10e21352f18c5.luc`
 - `929f6dbc83f6d3b65dab91f1efa4aacb.luc`
 - `b4a1d8ccc0c60e24e909f01c247f0a0f.luc`

Fortunately, installing TeXlive with `faketime -f` seems to pin both UUIDs and timestamps.

