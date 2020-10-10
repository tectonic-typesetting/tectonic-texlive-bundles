#! /bin/bash
# Copyright 2016-2020 the Tectonic Project.
# Licensed under the MIT License.

image_name=tectonic-texlive-bundler
bundler_cont_name=tectonic-bld-cont
source_dir="$(cd $(dirname "$0") && pwd)"
state_dir=$(pwd)/state # symlink here!

set -e

if [ -z "$1" -o "$1" = help ] ; then
    echo "You must supply a subcommand. Subcommands are:

build-image       -- Create or update the bundler Docker image.
bundler-bash      -- Run a shell in a temporary bundler container.
make-installation -- Install TeXLive into a new directory tree.
make-base-zipfile -- Make a Zip file of a standardized base TeXLive installation.
update-containers -- Rebuild the TeXLive \"container\" files.
zip2itar          -- Convert a bundle from Zip format to indexed-tar format.

"
    exit 1
fi

command="$1"
shift


function die () {
    echo >&2 "error:" "$@"
    exit 1
}


function build_image () {
    tag=$(date +%Y%m%d)
    docker build -t $image_name:$tag docker-image/
    docker tag $image_name:$tag $image_name:latest
}


function bundler_bash () {
    [ -d $state_dir/repo ] || die "no such directory $state_dir/repo"
    exec docker run -it --rm -v $source_dir:/source:ro,z -v $state_dir:/state:rw,z $image_name bash
}


function make_installation () {
    bundle_dir="$1"
    shift

    if [ ! -f "$bundle_dir/bundle.cfg" ] ; then
        die "usage: $0 make-installation <bundle-dir> [...]"
    fi

    bundle_dir="$(cd "$bundle_dir" && pwd)"

    [ -d $state_dir/repo ] || die "no such directory $state_dir/repo"

    exec docker run -it --rm -v $source_dir:/source:ro,z -v $state_dir:/state:rw,z -v $bundle_dir:/bundle:rw,z $image_name \
        python /source/scripts/make-installation.py "$@"
}


function make_base_zipfile () {
    zip="$1"

    if [ -z "$zip" ] ; then
        die "usage: $0 make-base-zipfile <output-zip-filename>"
    fi

    bundle_id=tlextras2018
    shift

    # First, TeXLive package installation.

    installdir=$(make_installation \
         collection-basic \
         collection-bibtexextra \
         collection-fontsextra \
         collection-fontsrecommended \
         collection-humanities \
         collection-latexextra \
         collection-latexrecommended \
         collection-latex \
         collection-luatex \
         collection-mathscience \
         collection-music \
         collection-pictures \
         collection-plaingeneric \
         collection-publishers \
         collection-xetex \
         collection-langarabic \
         collection-langchinese \
         collection-langcjk \
         collection-langcyrillic \
         collection-langczechslovak \
         collection-langenglish \
         collection-langeuropean \
         collection-langfrench \
         collection-langgerman \
         collection-langgreek \
         collection-langitalian \
         collection-langjapanese \
         collection-langkorean \
         collection-langother \
         collection-langpolish \
         collection-langportuguese \
         collection-langspanish
    )

    # Some manual fiddles for format file generation

    cp extras/$bundle_id/* $installdir/texmf-dist/

    # Finally, turn it all into a Zip file.

    ./bundler/make-zipfile.py "$installdir" "$zip"
    rm -rf "$installdir"
}


function update_containers () {
    [ -d $state_dir/repo ] || die "no such directory $state_dir/repo"
    mkdir -p $state_dir/containers
    docker run --rm -v $state_dir:/state:rw,z $image_name update-containers
}


function zip2itar () {
    zipfile="$1"

    if [ ! -f "$zipfile" ] ; then
        die "no such input file \"$zipfile\""
    fi

    dir=$(cd $(dirname "$zipfile") && pwd)
    zipfull=$dir/$(basename "$zipfile")
    tarfull=$dir/$(basename "$zipfile" .zip).tar
    echo "Generating $tarfull ..."
    cd $(dirname $0)/zip2tarindex
    exec cargo run --release -- "$zipfull" "$tarfull"
}


# Dispatch subcommands.

case "$command" in
    build-image)
        build_image "$@" ;;
    bundler-bash)
        bundler_bash "$@" ;;
    make-installation)
        make_installation "$@" ;;
    make-base-zipfile)
        make_base_zipfile "$@" ;;
    update-containers)
        update_containers "$@" ;;
    zip2itar)
        zip2itar "$@" ;;
    *)
        echo >&2 "error: unrecognized command \"$command\"."
        exit 1 ;;
esac
