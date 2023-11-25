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
- GNU `patch` and `diff`
- An installation of [Docker](https://www.docker.com/).
- A [TeXlive iso](https://tug.org/texlive/acquire-iso.html). Different bundles need different TeXlive versions.
- A Rust toolchain if you want to create “indexed tar” bundles. You don’t
  need Rust if you want to create a bundle and test it locally.

This repo also contains a `shell.nix` with pinned versions that contains all dependencies.








## Bundles:
Each directory in `./bundles` is a *bundle specification* which contains everything we need to reproducibly build a bundle.\
See [`./bundles/README.md`](./bundles/README.md) for details.

The following bundles are available:
 - `texlive/2022.0r0`: directly copied from the bundle in `master`. \
 Uses `texlive-2022.0r0`, and is probably broken.

 - `texlive2023-nopatch`: based on `texlive2023-20230313`.









## Build Process:
Before building any bundles, acquire a [TeXlive iso](https://tug.org/texlive/acquire-iso.html) with a version that matches the bundle you want to build. `build.sh` checks the hash of this file when you run `install`.

To build a bundle, run the following jobs. These **must** be run in order!

 - `./build.sh container`: builds the docker container from `./docker`
 - `./build.sh <bundle> install <iso>`: installs TeXLive to `./build/install/`
 - `./build.sh <bundle> content`: assemble all files into a bundle at `./build/output/content`.\
  This will delete all bundles in `output/<bundle>/`, move them elsewhere if you still need them.

Once `./build/output/content` has been created, run any of the following commands to package the bundle:

 - `./build.sh <bundle> zip`: create a zip bundle from the content directory.\
  Zip bundles can only be used locally, they may not be hosted on the web.

 - `./build.sh <bundle> itar`: create an indexed tar bundle from the content directory. \
 These cannot be used locally, itar bundles must be used as web bundles. \
 If you want to host your own, you'll need to put `bundle.tar` and `<bundle>.tar.sha256sum` under the same url.

`build.sh` also provides a few shortcuts:
 - `./build.sh <bundle> all <iso>`: Runs all the above jobs, *including* a full re-install.
 - `./build.sh <bundle> most <iso>`: Runs all jobs except `container` and `install`.
 - `./build.sh <bundle> package`: Runs `zip` and `itar`. Assumes `content` has already been run.



### Build Notes & Troubleshooting:
 - The `install` job could take a while. `tail -f` its log file to watch progress.
 - `install` will fail if your iso hash does not match the hash of the iso the bundle was designed for.\
 This may be overriden by replacing `./build.sh <bundle> install <iso>` with `./build.sh <bundle> forceinstall <iso>`.
 - the `install` job occasionally throws the following error: `mount: /iso-mount: failed to setup loop device for /iso.img.`\
 Run the job again, it should work. We don't yet know why this happens.
 - You do not need to run `install` every time you change a bundle. In fact, the contents of TeXlive installations should NEVER change. You only need to install each version of TeXlive once.\
 If you're building multiple bundles from the same TeXLive version, you could install once then copy & rename that installation to save time. Automating this would add a bit of needless complexity to the build process, but we may implement it later.






## Output Files


**`./build.sh <bundle> content` produces the following:**
 - `./build/output/<bundle>/content`: contains all bundle files. This directory also contains some metadata:
   - `content/INDEX`: each line of this file maps a filename in the bundle to a full path. Duplicate filenames are included.
   - `content/SHA256SUM`: a hash of this bundle's contents.
   - `content/TEXLIVE-SHA265SUM`: a hash of the TeXlive image used to build this bundle.
 - `search-report`: debug file. Lists all filenames that will be resolved alphabetically.
 - `file-hashes`: debug file. Indexes the contents of the bundle. Used to find which files differ between two builds.
  `file-hashes` and `content/SHA265SUM` are generated in roughly the same way, so the `file-hashes` files from two different bundles should match if and only if the two bundles have the same sha256sum


**`./build.sh <bundle> zip` produces the following:**
 - `<bundle>.zip`: the main zip bundle



**`./build.sh <bundle> itar` produces the following:**\
Note that both `<bundle>.tar` and `<bundle>.tar.index.gz` are required to host a web bundle.
 - `<bundle>.tar`: the tar bundle
 - `<bundle>.tar.index.gz`: the (compressed) tar index, with format `<file> <start> <len>`\
 This tells us that the first bit of `<file>` is at `<start>`, and the last is at `<start> + <len> - 1`.\
 You can extract a file from a local bundle using `dd if=file.tar ibs=1 skip=<start> count=<len>`\
 Or from a web bundle with `curl -r <start>-<start>+<len> https://url.tar`







## Reproducibility
The `SHA256HASH` stored in each bundle should stay the same between builds. \
Below is a list of "problem files" that have made bit-perfect rebuilds difficult in the past:

 - The following contain a timestamp:
   - `fmtutil.cnf`
   - `mf.base`
   - `updmap.cfg`
 - The following contain a UUID: (Most of these have a UUID *and* a timestamp)
   - `357744afc7b3a35aafa10e21352f18c5.luc`
   - `929f6dbc83f6d3b65dab91f1efa4aacb.luc`
   - `b4a1d8ccc0c60e24e909f01c247f0a0f.luc`

Fortunately, installing TeXlive with `faketime -f` seems to pin both UUIDs and timestamps.\
The date each bundle is built at is defined in its specification.

