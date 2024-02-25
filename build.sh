#!/usr/bin/env bash

this_dir="$(pwd)"
build_dir="${this_dir}/build"

# Print relative path.
# Only used for pretty printing.
function relative() {
	echo "./$(realpath --relative-to="${this_dir}" "${1}")"
}

# Extract the TeXLive tarball into /build/texlive.
# Arguments:
#	$1: source tarball
function extract_texlive() {
	local tar_file="${1}"

	if [[ "$tar_file" == "" ]]; then
		echo "You must provide a texlive image to run this job."
		exit 1
	fi

	if [[ ! -f "$tar_file" ]]; then
		echo "TeXlive iso $(relative "${tar_file}") doesn't exist!"
		exit 1
	fi

	local texlive_dir="${build_dir}/texlive/${tar_file%.tar}"


	mkdir -p "${texlive_dir}"
	chmod a+w -R "${texlive_dir}"
	# Remove target dir if it already exists
	if [[ ! -z "$(ls -A "${texlive_dir}")" ]]; then
		echo "Target directory is $(relative "${texlive_dir}")"
		for i in {5..2}; do
			echo -en "[WARNING] Target directory isn't empty, deleting in $i seconds...\r"
			sleep 1
		done
		echo -en "[WARNING] Target directory isn't empty, deleting in 1 second... \r"
		sleep 1

		echo -e "\nRunning \`rm -drf "${texlive_dir}"\`"
		rm -drf "${texlive_dir}"
		echo ""
	fi
	mkdir -p "${texlive_dir}"

	# We store this tar hash inside the target directory, and inside each bundle.

	local tar_hash=$(
		pv -N "Hashing TeXLive tar" -berw 60 "${tar_file}" | \
		sha256sum -b - | awk '{ print $1 }'
	)
	echo "Done: ${tar_hash}"


	pv -N "Extracting tarball" -berw 60 "${tar_file}" | \
		tar -x \
			--directory="${texlive_dir}" \
			--strip-components=2 \
			"${tar_file%.tar}/texmf-dist"


	if [[ $? != 0 ]]; then
		echo "TeXLive extraction failed"
		exit 1
	fi

	# Record iso hash
	echo "${tar_hash}" > "${texlive_dir}/TEXLIVE-SHA256SUM"
	chmod a-w -R "${texlive_dir}"

	echo ""
}


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
	"extract")
		extract_texlive "${1}"
	;;

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