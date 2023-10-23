# Tectonic TeXLive Bundle Builder

This repository contains scripts for building “bundles” for
[Tectonic](https://tectonic-typesetting.github.io).

*You do not need this repository to build Tectonic.* You only need these scripts
if you want to make your own bundles of TeX files.


## Prerequisites

To use these tools, you will need:

- An installation of [Docker](https://www.docker.com/).
- A TeXlive installation [iso](https://tug.org/texlive/acquire-iso.html)
- A Rust toolchain if you want to create “indexed tar” bundles. You don’t
  need Rust if you want to create a bundle and test it locally.


Output files consist of the following:
 - `./build/install`: TeXlive installation dir; intermedate files.
 - `./build/out`: finished bundles and support files


## Bundles:
The following bundles are available:
 - `full`: all of TeXlive, plus a few patches. Currently broken.\
 Uses `texlive-2022.0r0`.

 - `unpatched`: like `full`, but with no patches. Should work well.\
 Uses `texlive-2023.0r0`.


## Build Process:
Before building any bundles, acquire a [TeXlive iso](https://tug.org/texlive/acquire-iso.html) with a version that matches the bundle you want to build. Mount or copy its contents to `/build/iso`.

`./build.sh` handles the build process. The simplest way to use it is `./build $bundle all`,
which executes the following jobs in order:

 - **container:** builds the docker container from `./docker`
 - **install:** installs TeXLive to `./build/install/`
 - **zip:** creates a zip bundle
 - **itar:** converts that zip to an indexed tar bundle

Each of the steps above requires the previous steps. You may execute them one-by-one as follows. You can also create a bundle manually by reading `build.sh` and running `./build.sh <bundle> shell`.
```sh
./build.sh $bundle container
./build.sh $bundle install
./build.sh $bundle zip
./build.sh $bundle itar
```
There's no reason to do either of these unless you're debugging a bundle. `./build.sh $bundle all` should suffice for most cases.