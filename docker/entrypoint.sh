#!/usr/bin/env bash


function check_iso_hash () {
	source /bundle/bundle.sh

	if [[ "${bundle_texlive_hash}" == "" ]]; then
		echo "Not checking hash, bundle doesn't provide one."
		echo "Continuing..."
		sleep 2
		exit 0
	fi

	echo "Checking iso hash against bundles/${bundle_name}..."

	hash=$( sha256sum -b "/iso.img" | awk '{ print $1 }' )

	if [[ "${hash}" == "${bundle_texlive_hash}" ]]; then
		echo "OK: hash matches."
	else
		echo "Error: checksums do not match."
		echo ""
		echo "Got       $hash"
		echo "Expected  $bundle_texlive_hash"
		exit 1
	fi
}

# Install texlive into $1.
# Should be an absolute path.
function install () {
	source /bundle/bundle.sh

	mkdir /iso-mount
	mount /iso.img /iso-mount

	# Load profile and update destination paths
	profile=$(mktemp)
	sed -e "s|@dest@|/install|g" /bundle/tl-profile.txt > "${profile}"

	# Install texlive
	echo "It is $(date +%H:%M:%S)"
	echo "Installing TeXlive, this may take a while... (~15 minutes)"
	echo "Logs are streamed to build/install/${bundle_name}/tl-install.log"
	echo "Warnings will be printed below."
	echo ""

	cd /iso-mount
	rm -f "/install/tl-install.log"
	./install-tl -profile "${profile}" > "/install/tl-install.log"
	result="$?"

	echo "Done, cleaning up..."
	echo ""
	echo ""

	# Cleanup
	cd /
	umount /iso-mount
	rm -d /iso-mount
	rm "${profile}"
	chown $HOSTUID:$HOSTGID -R "/install"
	
	# Throw an error install failed
	# (otherwise, build.sh will not stop)
	if [[ $result != 0 ]]; then
		exit 1
	fi
}


# Make a zip bundle using an existing installation
function makezip () {
	python3 "/scripts/make-zipfile.py" $@
	chown $HOSTUID:$HOSTGID -R "/output"
}


command="$1"
shift

if [ "$command" = check_iso_hash ] ; then
	check_iso_hash
elif [ "$command" = install ] ; then
	install
elif [ "$command" = makezip ] ; then
	makezip $@
else
	echo "$0: unrecognized command \"$command\"."
	exit 1
fi
