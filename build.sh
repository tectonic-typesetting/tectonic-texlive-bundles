#!/usr/bin/env bash

image_name="rework-bundler"
build_dir="$(pwd)/build"
iso_dir="$(pwd)/build/iso"

target_bundle="${1#"bundles/"}" # Remove optional "bundles/" prefix
shift
job="${1}"
shift


function help () {
	cat << EOF

Usage: ./build.sh <bundle> <job>

Where <bundle> is a subpath of ./bundles
and <job> is one of the following:
	- shell: run a debug shell
	- all: run complete build process.

	- container: build docker image
	- install: install texlive
	- zip: create zip bundle
	- itar: create itar bundle
These four commands produce a complete build.
Each requires results the previous command.

EOF

	exit 0
}





# Make sure $job is valid
if [[
	"${target_bundle}" == "" ||
	"${job}" == "" ||
	! "$job" =~ ^(all|shell|bash|container|install|zip|itar)$
]] ; then
	help
fi


# Load and check bundle metadata
bundle_dir="$(realpath "bundles/${target_bundle}")"
if [ ! -f "$bundle_dir/bundle.sh" ] ; then
	echo >&2 "[ERROR] \`$bundle_dir\` has no bundle.sh, cannot proceed."
	exit 1
fi
source "${bundle_dir}/bundle.sh"
if [[
	-z ${bundle_name+x} ||
	-z ${bundle_texlive_file+x} ||
	-z ${bundle_texlive_hash+x}
]] ; then
	echo >&2 "[ERROR] Bundle config is missing values, check bundle.sh"
	exit 1
elif [ "${target_bundle}" != "${bundle_name}" ] ; then
	echo >&2 "[ERROR] \$bundle_name does not match folder name."
	exit 1
fi
unset target_bundle



install_dir="${build_dir}/install/${bundle_name}"
output_dir="${build_dir}/output/${bundle_name}"
# Must match path in make-zipfile.py
zip_path="${output_dir}/${bundle_name}.zip"

mkdir -p "${install_dir}"
mkdir -p "${output_dir}"

if [ ! -d $iso_dir ]; then 
	echo >&2 "[ERROR] Cannot start: no directory ${iso_dir}"
	exit 1
fi

docker_args=(
	-e HOSTUID=$(id -u)
	-e HOSTGID=$(id -g)
	-e bundle_name="${bundle_name}"
	-e bundle_texlive_version="${bundle_texlive_version}"
	-e bundle_texlive_hash="${bundle_texlive_hash}"
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



# Job implementations are below
# (In the order we need to run them)


# Run a shell in our container
# Only used to debug the build process.
if [[ "${job}" == "shell" || "${job}" == "bash" ]]; then
	docker run -it --rm "${docker_args[@]}" $image_name bash
	exit 0
fi

# Build the docker container in ./docker-image
if [[ "${job}" == "all" || "${job}" == "container" ]]; then
	tag=$(date +%Y%m%d)
	docker build -t $image_name:$tag docker-image/
	docker tag $image_name:$tag $image_name:latest
fi

# Install texlive in /build/install using our container
if [[ "${job}" == "all" || "${job}" == "install" ]]; then
	docker run -it --rm "${docker_args[@]}" $image_name install
fi

# Make a zip bundle from a texlive installation
if [[ "${job}" == "all" || "${job}" == "zip" ]]; then
	docker run -it --rm "${docker_args[@]}" $image_name makezip
fi

# Convert zip bundle to an indexed tar bundle
if [[ "${job}" == "all" || "${job}" == "itar" ]]; then
	tar_path="${output_dir}/$(basename "$zip_path" .zip).tar"
	echo "Generating $tar_path ..."
	cd $(dirname $0)/zip2tarindex
	exec cargo run --release -- "$zip_path" "$tar_path"
fi