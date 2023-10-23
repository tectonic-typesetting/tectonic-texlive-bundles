#!/usr/bin/env bash

image_name="rework-bundler"
build_dir="$(pwd)/build"

target_bundle="${1#"bundles/"}" # Remove optional "bundles/" prefix
shift
job="${1}"
shift
iso_name="${1}"
iso_file="$(realpath "${iso_name}")"
shift

function help () {
	cat << EOF

Usage: ./build.sh <bundle> <job> <iso>

Where <bundle> is a subpath of ./bundles,
<job> is one of the following:
<iso> is a TeXlive dvd image.

Jobs:
	- shell: run a debug shell
	- all: run complete build process.

	- container: build docker image
	- install: install texlive in docker
	- forceinstall: install texlive, but don't check hash
	- zip: create a zip bundle
	- itar: create an itar bundle

container, install, zip, and itar produce a complete build.
Each requires results from the previous command.

EOF

	exit 0
}

function relative() {
	echo "./$(realpath --relative-to="$(pwd)" "${1}")"
}



# Build the docker container in ./docker
if [[ "${job}" == "all" || "${job}" == "container" ]]; then
	tag=$(date +%Y%m%d)
	docker build -t $image_name:$tag docker/
	docker tag $image_name:$tag $image_name:latest

	if [[ "${job}" == "container" ]]; then
		exit 0
	fi
	echo ""
fi


# Make sure $args are valid
if [[
	"${target_bundle}" == "" ||
	"${job}" == "" ||
	"${iso_name}" == "" ||
	! "$job" =~ ^(all|shell|bash|container|install|forceinstall|zip|itar)$
]] ; then
	help
fi


# Load and check bundle metadata
bundle_dir="$(realpath "bundles/${target_bundle}")"
if [ ! -f "$bundle_dir/bundle.sh" ] ; then
	echo >&2 "[ERROR] $(relative "${bundle_dir}") has no bundle.sh, cannot proceed."
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
	echo >&2 "[ERROR] Cannot start: no directory $(relative "${iso_dir}")"
	exit 1
fi

# docker arguments.
# We mount an iso inside the container, so we need
# to be privileged.
docker_args=(
	--privileged
	-e HOSTUID=$(id -u)
	-e HOSTGID=$(id -g)
	-v "$iso_file":/iso.img:ro,z
	-v "$install_dir":/install:rw,z
	-v "$output_dir":/output:rw,z
	-v "$bundle_dir":/bundle:ro,z
)






# Job implementations are below
# (In the order we need to run them)


# Run a shell in our container
# Only used to debug the build process.
if [[ "${job}" == "shell" || "${job}" == "bash" ]]; then
	docker run -it --rm "${docker_args[@]}" $image_name bash
	exit 0
fi


# Install texlive in /build/install using our container
if [[ "${job}" == "all" || "${job}" == "install" || "${job}" == "forceinstall" ]]; then


	if [[ ! -z "$(ls -A "${install_dir}")" ]]; then
		echo "Install directory is $(relative "${install_dir}")"
		for i in {5..2}; do
			echo "[WARNING] Install directory isn't empty, deleting in $i seconds..."
			sleep 1
		done
		echo "[WARNING] Install directory isn't empty, deleting in 1 second..."
		sleep 1

		rm -drf "${install_dir}/*"
		echo "Ran \`rm -drf "${install_dir}/*"\`"
		echo ""
	fi
	
	# Check texlive hash
	if [[ "${job}" != "forceinstall" ]]; then
		docker run -it --rm "${docker_args[@]}" $image_name check_iso_hash
		if [[ $? != 0 ]]; then
			exit 1
		fi
		echo ""
	fi

	docker run -it --rm "${docker_args[@]}" $image_name install
	echo ""
fi

# Make a zip bundle from a texlive installation
if [[ "${job}" == "all" || "${job}" == "zip" ]]; then

	if [ ! -z "$(ls -A "${output_dir}")" ]; then
		echo "Output directory is $(relative "${output_dir}")"
		for i in {5..2}; do
			echo "[WARNING] Output directory isn't empty, deleting in $i seconds..."
			sleep 1
		done
		echo "[WARNING] Output directory isn't empty, deleting in 1 second..."
		sleep 1

		rm -f "${output_dir}/*"
		echo "Ran \`rm -f "${output_dir}/*"\`"
		echo ""
	fi

	docker run -it --rm "${docker_args[@]}" $image_name makezip "$bundle_name"
	echo ""
fi

# Convert zip bundle to an indexed tar bundle
if [[ "${job}" == "all" || "${job}" == "itar" ]]; then
	tar_path="${output_dir}/${bundle_name}.tar"
	rm -f "$tar_path"

	echo "Generating $(relative "${tar_path}")..."
	cd $(dirname $0)/zip2tarindex
	exec cargo run --release -- "$zip_path" "$tar_path"
	echo ""
fi