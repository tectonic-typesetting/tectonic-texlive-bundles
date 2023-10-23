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



command="$1"
shift

if [ "$command" = bash ] ; then
	exec bash "$@"
elif [ "$command" = python ] ; then
	exec python3 "$@"
elif [ "$command" = install ] ; then
	install
else
	echo "$0: unrecognized command \"$command\"."
	exit 1
fi
