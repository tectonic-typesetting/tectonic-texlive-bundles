#!/usr/bin/env bash

image_name="rework-bundler"
build_dir="$(pwd)/build"



function help () {
	echo "Invalid build command."
	echo "Usage: ./build.sh <bundle> <job> <iso>"
	echo "See README.md for detailed documentation."
	exit 1
}

function relative() {
	echo "./$(realpath --relative-to="$(pwd)" "${1}")"
}



# Set up and check arguments.
if [[ "${1}" == "container" ]]; then
	# "container" is a special case, since it takes no arguments.
	# Note that `.build.sh <bundle> container` works and has the
	# same effect as `./build.sh container`

	job="container"
	target_bundle=""
	iso_file=""
else
	# Which bundle specification are we building?
	# This is a path to a bundle directory, probably one in ./bundles
	bundle_dir="$(realpath "${1}")"
	shift

	# What do we want to do?
	job="${1}"
	shift

	# The image to build from.
	# This arg is optional for some jobs.
	iso_file="${1}"
	if [[ "${iso_file}" != "" ]] ; then
		iso_file="$(realpath "${iso_file}")"
	fi
	shift

	# Make sure args are valid.
	# We don't check iso_file here, since it's only required for some jobs.
	if [[
		"${bundle_dir}" == "" ||
		"${job}" == "" ||
		! "$job" =~ ^(all|shell|bash|container|install|forceinstall|zip|itar)$
	]] ; then
		help
	fi

	# Load and check bundle metadata
	if [ ! -f "$bundle_dir/bundle.sh" ] ; then
		echo >&2 "[ERROR] $(relative "${bundle_dir}") has no bundle.sh, cannot proceed."
		exit 1
	fi
	source "${bundle_dir}/bundle.sh"
	if [[
		-z ${bundle_name+x} ||
		-z ${bundle_texlive_hash+x} ||
		-z ${bundle_faketime+x} ||
		-z ${bundle_result_hash+x}
	]] ; then
		echo >&2 "[ERROR] Bundle config is missing values, check bundle.sh"
		exit 1
	fi
	unset target_bundle


	# Set up paths
	install_dir="${build_dir}/install/${bundle_name}"
	output_dir="${build_dir}/output/${bundle_name}"
	# Must match path in make-zipfile.py
	zip_path="${output_dir}/${bundle_name}.zip"
fi


function needs_iso() {
	if [[ "$iso_file" == "" ]]; then
		echo "You must provide a texlive image to run this job."
		exit 1
	fi

	if [[ ! -f "$iso_file" ]]; then
		echo "TeXlive iso $(relative "${iso_file}") doesn't exist!"
		exit 1
	fi
}



# Build the docker container in ./docker
function container() {
	local tag=$(date +%Y%m%d)
	docker build -t $image_name:$tag docker/
	docker tag $image_name:$tag $image_name:latest

	if [[ "${job}" == "container" ]]; then
		exit 0
	fi
	echo ""
}





# Job implementations are below
# (In the order we need to run them)




# Run a shell in our container
# Only used to debug the build process.
function shell() {
	needs_iso
	mkdir -p "${install_dir}"

	local docker_args=(
		--privileged
		-e HOSTUID=$(id -u)
		-e HOSTGID=$(id -g)
		-v "$iso_file":/iso.img:ro,z
		-v "$install_dir":/install:rw,z
		-v "$bundle_dir":/bundle:ro,z
	)

	docker run -it --rm "${docker_args[@]}" $image_name bash
	exit 0
}



# Install texlive in /build/install using our container
# If $1 is "nohash", don't check iso hash before build.
function install() {
	needs_iso

	local docker_args=(
		--privileged
		-e HOSTUID=$(id -u)
		-e HOSTGID=$(id -g)
		-v "$iso_file":/iso.img:ro,z
		-v "$install_dir":/install:rw,z
		-v "$bundle_dir":/bundle:ro,z
	)

	# Check texlive iso hash
	if [[ "${1}" != "nohash" ]]; then
		if [[ "${bundle_texlive_hash}" == "" ]]; then
			echo "Not checking TeXlive hash, bundle doesn't provide one."
			echo "Continuing..."
			sleep 2
			exit 0
		else
			echo "Checking iso hash against $(relative "${bundle_dir}")..."
			local hash=$( sha256sum -b "$iso_file" | awk '{ print $1 }' )
			if [[ "${hash}" == "${bundle_texlive_hash}" ]]; then
				echo "OK: hash matches."
			else
				echo "Error: checksums do not match."
				echo ""
				echo "Got       $hash"
				echo "Expected  $bundle_texlive_hash"
				exit 1
			fi
			echo ""
		fi
	fi


	mkdir -p "${install_dir}"
	# Remove install dir if it already exists
	if [[ ! -z "$(ls -A "${install_dir}")" ]]; then
		echo "Install directory is $(relative "${install_dir}")"
		for i in {5..2}; do
			echo "[WARNING] Install directory isn't empty, deleting in $i seconds..."
			sleep 1
		done
		echo "[WARNING] Install directory isn't empty, deleting in 1 second..."
		sleep 1

		echo "Running \`rm -drf "${install_dir}"\`"
		rm -drf "${install_dir}"
		echo ""
	fi
	mkdir -p "${install_dir}"


	echo "It is $(date +%H:%M:%S)"
	docker run -it --rm "${docker_args[@]}" $image_name install


	if [[ $? != 0 ]]; then
		echo "Install failed"
		exit 1
	fi

	echo ""
}





# Make a zip bundle from a texlive installation
function make_zip() {
	needs_iso

	mkdir -p "${output_dir}"
	if [ ! -z "$(ls -A "${output_dir}")" ]; then
		echo "Output directory is $(relative "${output_dir}")"
		for i in {5..2}; do
			echo "[WARNING] Output directory isn't empty, deleting in $i seconds..."
			sleep 1
		done
		echo "[WARNING] Output directory isn't empty, deleting in 1 second..."
		sleep 1

		echo "Running \`rm -drf "${output_dir}"\`"
		rm -drf "${output_dir}"
		echo ""
	fi
	mkdir -p "${output_dir}"


	echo "Hashing TeXlive iso..."
	echo "Make sure this is the same file you provided to \`install\`!"
	sleep 1
	local hash=$( sha256sum -b "$iso_file" | awk '{ print $1 }' )
	echo "Done: ${hash}"
	echo ""

	python3 make-zipfile.py "${bundle_dir}" "${hash}"
	if [[ $? != 0 ]]; then
		echo "Zip failed"
		exit 1
	fi
	echo ""

	# Check zip hash
	local zip_hash=$(unzip -p "${zip_path}" SHA256SUM)


	# Check result hash
	if [[ "${1}" != "nohash" ]]; then
		if [[ "${bundle_texlive_hash}" == "" ]]; then
			echo "Not checking result hash, bundle doesn't provide one."
			echo "Continuing..."
			sleep 2
			exit 0
		else
			if [ "${zip_hash}" != "${bundle_result_hash}" ]; then
				echo "[WARNING] zip hash does not match expected hash"
				echo "got      \"${zip_hash}\""
				echo "expected \"${bundle_result_hash}\""
				echo ""
				echo "Build has been stopped, but zip has been created."
				echo "Run \`./build.sh $(relative "${bundle_dir}") itar\` to continue."
				exit 1
			else
				echo "Zip done, hash matches."
			fi
		fi
	fi

	
}







# Convert zip bundle to an indexed tar bundle
function make_itar() {
	mkdir -p "${output_dir}"

	if [[ ! -f "$zip_path" ]]; then
		echo "There is no zip file at $(relative "${zip_path}")."
		echo "Run \`./build.sh $(relative "${bundle_dir}") zip\` to make it."
		exit 1
	fi

	local tar_path="${output_dir}/${bundle_name}.tar"
	rm -f "$tar_path"

	echo "Generating $(relative "${tar_path}")..."
	cd $(dirname $0)/zip2tarindex
	exec cargo run --release -- "$zip_path" "$tar_path"
	echo ""
}




case "${job}" in

	"all")
		container
		install
		make_zip
		make_itar
	;;

	"shell" | "bash")
		shell
	;;

	"container")
		container
	;;

	"install")
		install
	;;

	"forceinstall")
		install nohash
	;;

	"zip")
		make_zip
	;;

	"itar")
		make_itar
	;;

	*)
		echo "Warning: unreachable code!"
		echo ""
		help
	;;
esac