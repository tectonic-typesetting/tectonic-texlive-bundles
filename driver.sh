#! /bin/bash
# Copyright 2016-2020 the Tectonic Project.
# Licensed under the MIT License.

image_name=tectonic-texlive-bundler
bundler_cont_name=tectonic-bld-cont
source_dir="$(cd $(dirname "$0") && pwd)"
state_dir=$(pwd)/state # symlink here!

docker_args=(
    -e HOSTUID=$(id -u)
    -e HOSTGID=$(id -u)
    -v "$source_dir":/source:ro,z
    -v "$state_dir":/state:rw,z
)

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

function require_repo() {
    [ -d $state_dir/repo ] || die "no such directory $state_dir/repo"
}

function use_bundle() {
    bundle_dir="$1"
    shift

    if [ ! -f "$bundle_dir/bundle.toml" ] ; then
        die "bundle directory \`$bundle_dir\` looks invalid (no bundle.toml)"
    fi

    bundle_dir="$(cd "$bundle_dir" && pwd)"
    docker_args+=(
        -v "$bundle_dir":/bundle:rw,z
    )
}

# Subcommands (alphabetical order):

function build_image () {
    tag=$(date +%Y%m%d)
    docker build -t $image_name:$tag docker-image/
    docker tag $image_name:$tag $image_name:latest
}


function bundler_bash () {
    exec docker run -it --rm "${docker_args[@]}" $image_name bash
}


function install_packages () {
    bundle_dir="$1"
    shift

    if [ -z "$bundle_dir" ] ; then
        die "usage: $0 install-packages <bundle-dir>"
    fi

    use_bundle "$bundle_dir"
    require_repo

    exec docker run -it --rm "${docker_args[@]}" $image_name \
        python /source/scripts/install-packages.py "$@"
}


function make_installation () {
    bundle_dir="$1"
    shift

    if [ -z "$bundle_dir" ] ; then
        die "usage: $0 make-installation <bundle-dir>"
    fi

    use_bundle "$bundle_dir"
    require_repo

    exec docker run -it --rm "${docker_args[@]}" $image_name \
        python /source/scripts/make-installation.py "$@"
}


function make_base_zipfile () {
    bundle_dir="$1"
    shift
    zip="$1"
    shift

    if [ -z "$zip" ] ; then
        die "usage: $0 make-base-zipfile <bundle-dir> <output-zip-filename>"
    fi

    use_bundle "$bundle_dir"

    echo XXX TODO COPY IN EXTRAS

    exec docker run -it --rm "${docker_args[@]}" $image_name \
        python /source/scripts/make-zipfile.py "$zip" "$@"
}


function update_containers () {
    require_repo
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
    install-packages)
        install_packages "$@" ;;
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
