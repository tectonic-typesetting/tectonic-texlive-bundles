# Tectonic Bundle Builder

This repository contains scripts for building “bundles” for
[Tectonic](https://tectonic-typesetting.github.io) based on [Norbert Preining’s
Git mirror](http://git.texlive.info/texlive/) of [the TeXLive Subversion
repository](http://tug.org/svn/texlive/).

*You do not need this repository to build Tectonic.* You only need these scripts
if you want to make your own bundle of TeX files.


## Prerequisites

To do this, you will need at a minimum:

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


## Creating the Zip bundle

Once you have created your TeXLive containers, the script
`make-zipfile.py` can compile them into a single master Zip file. The
operation `./driver.sh make-base-zipfile $DESTPATH` will do this for the
standard Tectonic base bundle, `tlextras`. It does so using the helper
`./driver.sh make-installation`.


## Creating the “indexed tar” bundle

For bundles to be hosted on the web, the operation `./driver.sh zip2itar` will
convert the resulting Zip file to the “indexed tar” format used for Web-based
bundles. **TODO**: this is not adequately documented at all.


# Copyright and Licensing

The infrastructure scripts in this repository are licensed under the MIT
License. Their copyright is assigned to the Tectonic Project.
