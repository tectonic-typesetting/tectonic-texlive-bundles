# Tectonic TeXLive Bundle Builder

This repository contains scripts for building “bundles” for
[Tectonic](https://tectonic-typesetting.github.io) based on [Norbert Preining’s
Git mirror](http://git.texlive.info/texlive/) of [the TeXLive Subversion
repository](http://tug.org/svn/texlive/).

*You do not need this repository to build Tectonic.* You only need these scripts
if you want to make your own bundle of TeX files based on the TeXLive sources.


## Prerequisites

To use these tools, you will need:

- An installation of [Docker](https://www.docker.com/).
- A checkout of the Preining TeXLive Git repository
  (`git://git.texlive.info/texlive.git`), placed or symlinked in a subdirectory
  named `state/repo` below this file. Be aware that this repository currently
  weighs in at **40 gigabytes**!

Data files associated with the staging process will land in other subdirectories
of `state/`.


## Getting started: creating the bundler image

The first step is to create a Docker container that will host most of the
computations — this promotes reproducibility and portability, regardless of what
kind of system you are using. To create this container, run:

```
./driver.sh build-image
```


## Creating TeXLive containers

The next step is to create TeXLive “containers” — which are different than
Docker containers. A *Docker* container is an encapsulated Linux machine that
provides a reproducible build environment. *TeXLive* containers are archives
containing the files associated with the various TeXLive packages.

To create TeXLive container files for all of the packages associated with your
TeXLive checkout, run:

```
./driver.sh update-containers
```

This will use the Docker container to generate TeXLive container files in
`state/containers`. The script will furthermore copy those files to
`state/versioned`, altering the names to record the exact version of each
package. *Note that the results of this step will depend on what version of the
TeXLive tree you currently have checked out in `state/repo`.*


## Creating a TeXLive installation tree

**NOTE: this workflow is still evolving!**.

Run:

```
./driver.sh make-installation bundles/tlextras
./driver.sh install-packages bundles/tlextras
```


## Exporting to a Zip-format bundle

**NOTE: this workflow is still evolving!**.

Run:

```
./driver.sh make-zipfile bundles/tlextras
```

A local copy of this bundle file can be used with the `tectonic` command-line
program with the `-b` argument.


## Converting to an “indexed tar” bundle

**NOTE: this workflow is still evolving!**.

This step is needed to create a bundle that will be hosted on the web. Run:

```
./driver.sh make-itar bundles/tlextras
```

This will create both the `.tar` and the `.tar.index.gz` files that need to be
uploaded for use as a web bundle.


#### Copyright and Licensing

The infrastructure scripts in this repository are licensed under the MIT
License. Their copyright is assigned to the Tectonic Project.
