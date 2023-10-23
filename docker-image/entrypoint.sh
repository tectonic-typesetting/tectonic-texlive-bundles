#!/usr/bin/env bash


# Install texlive into $1.
# Should be an absolute path.
function install () {
	source /bundle/bundle.sh

	# Load profile and update destination paths
	profile=$(mktemp)
	sed -e "s|@dest@|/install|g" /bundle/tl-profile.txt > "${profile}"

	# Install texlive
	cd /iso
	./install-tl -profile "${profile}" | tee "/install/tl-install.log"

	# Cleanup
	rm "${profile}"
	chown $HOSTUID:$HOSTGID -R "/install"
}


# Make a zip bundle using an existing installation
function makezip () {
	exec python3 "/scripts/make-zipfile.py"
	chown $HOSTUID:$HOSTGID -R "/output"
}


command="$1"
shift

if [ "$command" = install ] ; then
	install
elif [ "$command" = makezip ] ; then
	makezip
else
	echo "$0: unrecognized command \"$command\"."
	exit 1
fi
