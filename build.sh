#!/usr/bin/env bash

build_dir="$(pwd)/build"


# Select files for this bundle
function select_files() {
	# cargo run needs an absolute path
	local bundle_dir="$(realpath "${1}")"

	cd "builder"
	cargo build --quiet --release
	cargo run --quiet --release -- \
		select "${bundle_dir}" "${build_dir}"
}

# Make a V1 ttb from the content directory
function make_ttbv1() {
	local bundle_dir="$(realpath "${1}")"

	(
		cd "builder"
		cargo build --quiet --release

		cargo run --quiet --release -- \
			build v1 "${bundle_dir}" "${build_dir}"
	)
}


# We use the slightly unusual ordering `./build.sh <arg> <job>`
# so that it's easier to change the job we're running on a bundle

case "${2}" in
	"select")
		select_files "${1}"
	;;

	"ttbv1")
		make_ttbv1 "${1}"
	;;

	*)
		echo "Invalid build command."
		echo "Usage: ./build.sh <bundle> <job>"
		echo "See README.md for detailed documentation."
		exit 1
	;;
esac