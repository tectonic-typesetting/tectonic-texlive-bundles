#!/usr/bin/env bash

image_name="tectonic-bundler"
this_dir="$(pwd)"
build_dir="${this_dir}/build"

# Print relative path.
# Only used for pretty printing.
function relative() {
	echo "./$(realpath --relative-to="${this_dir}" "${1}")"
}


# Load and check bundle metadata.
function load_bundle() {
	local bundle_dir="${1}"

	if [ ! -d "$bundle_dir" ]; then
		echo >&2 "[ERROR] $(relative "${bundle_dir}") doesn't exist, cannot proceed."
		exit 1
	fi
	if [ ! -f "$bundle_dir/bundle.sh" ]; then
		echo >&2 "[ERROR] $(relative "${bundle_dir}") has no bundle.sh, cannot proceed."
		exit 1
	fi
	source "${bundle_dir}/bundle.sh"
	if [[
		-z ${bundle_name+x} ||
		-z ${bundle_texlive_hash+x} ||
		-z ${bundle_texlive_name+x} ||
		-z ${bundle_result_hash+x}
	]] ; then
		echo >&2 "[ERROR] Bundle config is missing values, check bundle.sh"
		exit 1
	fi
}





# Job implementations are below
# (In the order we need to run them)
#
# These functions take no implicit parameters.
# All arguments are provided explicitly.




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
# Arguments:
#	$1: bundle specification
function select_files() {
	# cargo run needs an absolute path
	local bundle_dir="$(realpath "${1}")"
	load_bundle "${bundle_dir}"

	local texlive_dir="build/texlive/${bundle_texlive_name}"
	local output_dir="${build_dir}/output/${bundle_name}"

	if [[ ! -d "${texlive_dir}" ]]; then
		echo "TeXLive source for \"${bundle_texlive_name}\" doesn't exist."
		echo "You may have forgotten to run \`./bundle.sh extract\`"
		exit 1
	fi

	local tar_hash="$(cat "${texlive_dir}/TEXLIVE-SHA256SUM")"

	# Check texlive iso hash
	if [[ "${bundle_texlive_hash}" == "" ]]; then
		echo "Not checking TeXlive hash, bundle doesn't provide one."
		echo "Continuing..."
		sleep 1
	else
		echo "Checking extracted hash against $(relative "${bundle_dir}")..."
		if [[ "${tar_hash}" == "${bundle_texlive_hash}" ]]; then
			echo "OK: hash matches."
		else
			echo "Error: checksums do not match."
			echo ""
			echo "Got       $tar_hash"
			echo "Expected  $bundle_texlive_hash"
			echo ""
			echo "This is a critical error. Edit the bundle specification"
			echo "if you'd like to use a different file."
			exit 1
		fi
		echo ""
	fi

	mkdir -p "${output_dir}"
	if [ ! -z "$(ls -A "${output_dir}")" ]; then
		echo "Output directory is $(relative "${output_dir}")"
		for i in {5..2}; do
			echo -en "[WARNING] Output directory isn't empty, deleting in $i seconds...\r"
			sleep 1
		done
		echo -en "[WARNING] Output directory isn't empty, deleting in 1 second... \r"
		sleep 1

		echo -e "\nRunning \`rm -drf "${output_dir}"\`"
		rm -drf "${output_dir}"
		echo ""
	fi
	mkdir -p "${output_dir}"

	(
		cd "builder"
		cargo build --quiet --release

		cargo run --quiet --release -- \
			select "${bundle_dir}" "${build_dir}" "${bundle_texlive_name}" "${bundle_name}"
	)
	if [[ $? != 0 ]]; then
		echo "File selector failed"
		exit 1
	fi
	echo ""

	# Check content hash
	local content_hash=$(cat "${output_dir}/content/SHA256SUM")

	if [[ "${1}" != "nohash" ]]; then
		if [[ "${bundle_texlive_hash}" == "" ]]; then
			echo "Not checking content hash, bundle doesn't provide one."
			echo "Continuing..."
			sleep 2
			exit 0
		else
			if [ "${content_hash}" != "${bundle_result_hash}" ]; then
				echo "[WARNING] content hash does not match expected hash"
				echo "got      \"${content_hash}\""
				echo "expected \"${bundle_result_hash}\""
			else
				echo "File selection done, hash matches."
			fi
		fi
	fi
	echo ""
}

# Make a V1 ttb from the content directory
# Arguments:
#	$1: bundle specification
function make_ttbv1() {
	local bundle_dir="${1}"
	load_bundle "${bundle_dir}"
	local output_dir="${build_dir}/output/${bundle_name}"
	local ttb_path="${output_dir}/${bundle_name}.ttb"
	rm -f "${zip_path}"

	if [ -z "$(ls -A "${output_dir}/content")" ]; then
		echo "Bundle content directory doesn't exist at $(relative "${output_dir}/content")"
		echo "Cannot proceed. Run \`./build.sh $(relative "${bundle_dir}") content\`, then try again."
		exit 1
	fi

	if [[ -f "${ttb_path}" ]]; then
		echo "ttb bundle exists at $(relative "${ttb_path}"), removing."
		rm "${ttb_path}"
	fi

	(
		cd "builder"
		cargo build --quiet --release

		cargo run --quiet --release -- \
			build v1 "${output_dir}/content" "${ttb_path}"
	)
}


# We use the slightly unusual ordering `./build.sh <arg> <job>`
# so that it's easier to change the job we're running on a bundle

case "${2}" in


	# Single jobs
	"extract")
		extract_texlive "${1}"
	;;

	"content")
		select_files "${1}"
	;;

	"ttbv1" | "ttb1")
		make_ttbv1 "${1}"
	;;

	*)
		echo "Invalid build command."
		echo "Usage: ./build.sh <bundle> <job>"
		echo "See README.md for detailed documentation."
		exit 1
	;;
esac