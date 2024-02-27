# Tectonic Bundles

This repository contains tools for building bundles for
[Tectonic](https://tectonic-typesetting.github.io), each of which is a complete TeX distribution.

**You do not need this repository to build Tectonic.** \
You only need this if you want to make your own bundles of TeX files.


## Prerequisites
To use these tools, you will need:
- Cargo, Bash, `pv`, GNU `patch` and `diff`. Patch is called by `builder` while running `select`.
- A [TeXlive tarball](https://tug.org/texlive/acquire-tar.html).

The following bundles are available:
 - [`texlive2023`](./bundles/texlive2023): based on `texlive2023-20230313`.


## Build Process:
Before building any bundles, you'll need to download the prerequisite files.
Usually, this is a [TeXlive tarball](https://tug.org/texlive/acquire-tar.html) with a version that matches the bundle you want to build. See `bundle.toml` in the bundle you want to build, the version of TeXlive and a link to the tarball should
be provided.


To build a bundle, run the following:
 - `cd builder`
 - `cargo run -- --build-dir <working_directory> <path to bundle.toml>`

For example, `cargo run -- --build-dir ../build "../bundles/texlive2023/bundle.toml"` \
See `cargo run -- --help` for detailed information.

This runs the following jobs, in order. Individual jobs may be run by specifying `--job <job name>`.
 - `select`
 - `pack`

The contents of `<build dir>/content` may be inspected and edited after running `select`. \
This should only be used to debug bundles. The contents of this directory are documented [here](./doc/output.md).


## Extra Documentation
 - Each directory in [`./bundles`](./bundles/) is a bundle specification, documented [here](./doc/bundle.md).
 - Only one bundle format is currently supported, it is described in [`doc/formatspec-v1.md`](./doc/formatspec-v1.md).
 - This repository includes a few basic bundle [tests](./doc/tests.md).


