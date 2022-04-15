#! /bin/bash
# Copyright 2016-2022 the Tectonic Project.
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
    echo "You must supply a subcommand. Main subcommands are (in usual workflow order):

  build-image         -- Create or update the bundler Docker image.
  update-containers   -- (Re)build the TeXLive \"container\" files.
  make-installation   -- Install TeXLive into a new directory tree.
  install-packages    -- Add packages to a TeXLive installation tree.
  get-vendor-pristine -- Extract vendor-pristine versions of patched files.
  make-zipfile        -- Make a Zip file from a TeXLive installation tree.
  make-itar           -- Convert a bundle from Zip format to indexed-tar format.

Also:

  bundler-bash      -- Run a shell in a temporary bundler container.
"
    exit 1
fi

command="$1"
shift


function die () {
    echo >&2 "error:" "$@"
    exit 1
}

if [ ! -d $state_dir ] ; then
    die "you must create or symlink a \"state\" directory at the path \"$state_dir\""
fi

function require_repo() {
    [ -d $state_dir/repo ] || die "no such directory $state_dir/repo"
}

function use_bundle() {
    local bundle_dir="$1"
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
    local tag=$(date +%Y%m%d)
    docker build -t $image_name:$tag docker-image/
    docker tag $image_name:$tag $image_name:latest
}


function bundler_bash () {
    exec docker run -it --rm "${docker_args[@]}" $image_name bash
}


function get_vendor_pristine () {
    local bundle_dir="$1"
    shift || die "usage: $0 get-vendor-pristine <bundle-dir>"

    use_bundle "$bundle_dir"

    docker run -it --rm "${docker_args[@]}" $image_name \
        python /source/scripts/get-vendor-pristine.py "$@"

    # (ab)use this helper to get the artifacts directory:
    ziprel="$(docker run --rm "${docker_args[@]}" $image_name python /source/scripts/misc.py zip-relpath)"
    artifacts="$(dirname "$ziprel")"
    vname="$(basename "$artifacts")"
    cur_branch="$(git symbolic-ref --short -q HEAD)"

    echo
    echo "Now do something like the following:"
    echo
    echo "1) git status # confirm that tree and index are clean"
    echo "2) git switch vendor-pristine"
    echo "3) cp $artifacts/vendor-pristine/* $bundle_dir/patched/"
    echo "4) git add $bundle_dir/patched/"
    echo "5) git commit -m \"$bundle_dir: update vendored files for $vname\""
    echo "6) git switch $cur_branch"
    echo "7) git merge vendor-pristine # and resolve any conflicts"
}


function install_packages () {
    local bundle_dir="$1"
    shift || die "usage: $0 install-packages <bundle-dir>"

    use_bundle "$bundle_dir"
    require_repo

    docker run -it --rm "${docker_args[@]}" $image_name \
        python /source/scripts/install-packages.py "$@"

    echo
    echo "Next, you might want to run \`$0 make-zipfile $bundle_dir\`"
}


function make_installation () {
    local bundle_dir="$1"
    shift || die "usage: $0 make-installation <bundle-dir>"

    use_bundle "$bundle_dir"
    require_repo

    docker run -it --rm "${docker_args[@]}" $image_name \
        python /source/scripts/make-installation.py "$@"

    echo
    echo "Next, you might want to run \`$0 install-packages $bundle_dir\`"
}


function make_itar () {
    local bundle_dir="$1"
    shift || die "usage: $0 make-itar <bundle-dir>"

    use_bundle "$bundle_dir"

    ziprel="$(docker run --rm "${docker_args[@]}" $image_name python /source/scripts/misc.py zip-relpath)"
    dir=$(cd $(dirname "$ziprel") && pwd)
    zipfull=$dir/$(basename "$ziprel")
    tarfull=$dir/$(basename "$ziprel" .zip).tar
    echo "Generating $tarfull ..."
    cd $(dirname $0)/zip2tarindex
    exec cargo run --release -- "$zipfull" "$tarfull"
}


function make_zipfile () {
    local bundle_dir="$1"
    shift || die "usage: $0 make-zipfile <bundle-dir>"

    use_bundle "$bundle_dir"

    docker run -it --rm "${docker_args[@]}" $image_name \
        python /source/scripts/make-zipfile.py "$@"

    echo
    echo "Now you can test your bundle with commands like \`$(dirname $0)/tests/formats.py $bundle_dir\`"
}


function update_containers () {
    require_repo
    mkdir -p $state_dir/containers
    docker run --rm -v $state_dir:/state:rw,z $image_name update-containers
}


# Dispatch subcommands.

case "$command" in
    build-image)
        build_image "$@" ;;
    bundler-bash)
        bundler_bash "$@" ;;
    get-vendor-pristine)
        get_vendor_pristine "$@" ;;
    install-packages)
        install_packages "$@" ;;
    make-installation)
        make_installation "$@" ;;
    make-itar)
        make_itar "$@" ;;
    make-zipfile)
        make_zipfile "$@" ;;
    update-containers)
        update_containers "$@" ;;
    *)
        echo >&2 "error: unrecognized command \"$command\"."
        exit 1 ;;
esac
