#!/usr/bin/env bash

image_name="rework-bundler"
build_dir="$(pwd)/build"
iso_dir="$(pwd)/build/iso"
bundle_name="${1}"
shift
job="${1}"
shift




function die () {
	echo >&2 "error:" "$@"
	exit 1
}


function help () {
	cat << EOF

Usage: ./build.sh <bundle> <job>

Where <bundle> is a subpath of ./bundles
and <job> is one of the following:
	- shell: run a debug shell
	- all: run the following, in order

	- container: build docker image
	- install: install texlive
	- zip: create zip bundle
	- itar: create itar bundle
Each of the last four commands requires the previous.

EOF

	exit 0
}

if [[
	"${bundle_name}" == "" ||
	"${job}" == "" ||
	! "$job" =~ ^(all|shell|bash|container|install|zip|itar)$
]] ; then
	help
fi


# Check bundle path
bundle_dir="$(realpath "bundles/${bundle_name}")"
if [ ! -f "$bundle_dir/bundle.sh" ] ; then
	die "Bundle directory \`$bundle_dir\` looks invalid (no bundle.sh)"
fi


# Load and check bundle metadata
source "${bundle_dir}/bundle.sh"
if [ "${bundle_name}" != "${bn_name}" ] ; then
	die "[ERROR] Bundle name does not match folder name. Fix bundles/${bundle_name}/bundle.sh"
fi

install_dir="${build_dir}/install/${bn_name}-${bn_texlive_version}"
output_dir="${build_dir}/output/${bn_name}-${bn_texlive_version}"

mkdir -p "${install_dir}"
mkdir -p "${output_dir}"

[ -d $build_dir/iso ] || die "no such directory ${build_dir}/iso"
docker_args=(
	-e HOSTUID=$(id -u)
	-e HOSTGID=$(id -g)
	-e bn_name="${bn_name}"
	-e bn_texlive_version="${bn_texlive_version}"
	-e bn_texlive_hash="${bn_texlive_hash}"
	-v "$iso_dir":/iso:ro,z
	-v "$install_dir":/install:rw,z
	-v "$output_dir":/output:rw,z
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





if [[ "${job}" == "shell" || "${job}" == "bash" ]]; then
	docker run -it --rm "${docker_args[@]}" $image_name bash
	exit 0
fi




# Replaces ./driver.sh build-image
if [[ "${job}" == "all" || "${job}" == "container" ]]; then
	tag=$(date +%Y%m%d)
	docker build -t $image_name:$tag docker-image/
	docker tag $image_name:$tag $image_name:latest
fi





# We're building from an iso, so we skip ./driver.sh update-containers.
# Replaces ./driver.sh make-installation bundles/tlextras
if [[ "${job}" == "all" || "${job}" == "install" ]]; then
	docker run -it --rm "${docker_args[@]}" $image_name install
fi






# Skip ./driver.sh install-packages bundles/tlextras, since tl-install should do everything for us.
# Replaces ./driver.sh make-zipfile bundles/tlextras
if [[ "${job}" == "all" || "${job}" == "zip" ]]; then
	docker run -it --rm "${docker_args[@]}" $image_name python /scripts/make-zipfile.py
fi





# Replaces ./driver.sh make-itar bundles/tlextras
if [[ "${job}" == "all" || "${job}" == "itar" ]]; then
	ziprel="${output_dir}/${bn_name}-${bn_texlive_version}.zip"
	dir=$(cd $(dirname "$ziprel") && pwd)
	zipfull=$dir/$(basename "$ziprel")
	tarfull=$dir/$(basename "$ziprel" .zip).tar
	echo "Generating $tarfull ..."
	cd $(dirname $0)/zip2tarindex
	exec cargo run --release -- "$zipfull" "$tarfull"
fi