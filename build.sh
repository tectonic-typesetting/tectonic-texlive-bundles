#!/usr/bin/env bash
# Copyright 2016-2022 the Tectonic Project.
# Licensed under the MIT License.

image_name="rework-bundler"
build_dir="$(pwd)/build"
iso_dir="$(pwd)/build/iso"
bundle_name="${1}"
shift


function die () {
    echo >&2 "error:" "$@"
    exit 1
}

bundle_dir="$(cd "bundles/${bundle_name}" && pwd)"
if [ ! -f "$bundle_dir/bundle.sh" ] ; then
    die "bundle directory \`$bundle_dir\` looks invalid (no bundle.sh)"
fi

# Load bundle metadata
source "${bundle_dir}/bundle.sh"

if [ "${bundle_name}" != "${bn_name}" ] ; then
    die "[ERROR] Bundle name does not match folder name. Fix bundles/${bundle_name}/bundle.sh"
fi

docker_args=(
    -e HOSTUID=$(id -u)
    -e HOSTGID=$(id -g)
    -e bn_name="${bn_name}"
    -e bn_texlive_version="${bn_texlive_version}"
    -e bn_texlive_hash="${bn_texlive_hash}"
    -v "$iso_dir":/iso:ro,z
    -v "$build_dir":/build:rw,z
    -v "$bundle_dir":/bundle:ro,z
)





function check_hash () {
    bundle_name="${1}"
    file_name="${2}"
    source "bundles/${bundle_name}/bundle.sh"

    echo "Checking ${file_name} against bundles/${bundle_name}..."

    hash=$( sha256sum -b "${file_name}" | awk '{ print $1 }' )

    if [[ "${hash}" == "${bn_texlive_hash}" ]]; then
        echo "OK: hash matches."
    else
        echo "ERR: checksum does not match."
    fi
}







# Replaces ./driver.sh build-image

if true; then
    tag=$(date +%Y%m%d)
    docker build -t $image_name:$tag docker-image/
    docker tag $image_name:$tag $image_name:latest
fi


if [[ "$1" == "shell" || "$1" == "bash" ]]; then 
    docker run -it --rm "${docker_args[@]}" $image_name bash
    exit 0
fi



# We're building from an iso, so we skip ./driver.sh update-containers.
# Replaces ./driver.sh make-installation bundles/tlextras

if true; then
    [ -d $build_dir/iso ] || die "no such directory $build_dir/iso"
    docker run -it --rm "${docker_args[@]}" $image_name install "/build/installs"
fi






# Skip ./driver.sh install-packages bundles/tlextras, since tl-install should do everything for us.
# Replaces ./driver.sh make-zipfile bundles/tlextras

if true; then
    docker run -it --rm "${docker_args[@]}" $image_name python /scripts/make-zipfile.py
fi





# Replaces ./driver.sh make-itar bundles/tlextras

if true; then
    ziprel="$(docker run --rm "${docker_args[@]}" $image_name python /scripts/misc.py zip-relpath)"
    dir=$(cd $(dirname "$ziprel") && pwd)
    zipfull=$dir/$(basename "$ziprel")
    tarfull=$dir/$(basename "$ziprel" .zip).tar
    echo "Generating $tarfull ..."
    cd $(dirname $0)/zip2tarindex
    exec cargo run --release -- "$zipfull" "$tarfull"
fi