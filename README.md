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


## Documentation
 - Each directory in [`./bundles`](./bundles/) is a bundle specification, documented [here](./doc/bundle.md).
 - Only one bundle format is currently supported, described in [`doc/formatspec-v1.md`](./doc/formatspec-v1.md).
 - This repository includes a few basic bundle [tests](./doc/tests.md).





## Build Process:
Before building any bundles, acquire a [TeXlive tarball](https://tug.org/texlive/acquire-tar.html) with a version that matches the bundle you want to build. These are usually compressed, so you'll have to manually decompress the `.tar.gz` into a plain `.tar`. **`build.sh` expects an uncompressed `.tar` file!** It checks the hash of this file, and will refuse to work if that hash doesn't match.

The tarball you end up with should have a name of the form `texlive-YYYYMMDD-texmf.tar`. **Do not rename this file!**



To build a bundle, run the following jobs in order:
 - `./build.sh <tarball> extract`: extracts texlive into `./build/texlive/<version>` and computes hashes.
 - `./build.sh <bundle> select`: selects and patches extracted files into a bundle at `./build/output/<bundle>/content`.
 - `./build.sh <bundle> ttbv1`: create a [`ttbv1`](./doc/formatspec-v1.md) bundle from the content directory, to be used locally or on the web.

The contents of `./build/output/<bundle>/content` may be inspected and edited after running `select`. This should only be used to debug bundles, any bundle we publish should not need manual edits.
The contents of this directory are documented [here](./doc/output.md).