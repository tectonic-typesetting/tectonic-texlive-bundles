#!/usr/bin/env bash

# Install texlive into $1.
# Should be an absolute path.
function install () {
	source /bundle/bundle.sh

	mkdir /iso-mount
	mount /iso.img /iso-mount

	if [[ $? != 0 ]]; then
		exit 1
	fi

	# Load profile and update destination paths
	profile=$(mktemp)
	sed -e "s|@dest@|/install|g" /bundle/tl-profile.txt > "${profile}"

	# Install texlive
	echo "Installing TeXlive, this may take a while... (~15 minutes)"
	echo "Logs are streamed to build/install/${bundle_name}/tl-install.log"
	echo "Warnings will be printed below."
	echo ""

	cd /iso-mount
	rm -f "/install/tl-install.log"
	faketime -f "${bundle_faketime}" ./install-tl -profile "${profile}" > "/install/tl-install.log"
	result="$?"

	#if [[ $result != 0 ]]; then
	#	echo "Build failed, cleaning up..."
	#else
	#	echo "Done, cleaning up..."
	#fi
	echo "Done, cleaning up..."
	echo ""
	echo ""

	# Cleanup
	cd /
	umount /iso-mount
	rm -d /iso-mount
	rm "${profile}"
	chown $HOSTUID:$HOSTGID -R "/install"

	# Hacky check for install success.
	# texlive sometimes returns 1 when installation succeeds with minor warnings.
	if ! grep -Fxq "Welcome to TeX Live!" "/install/tl-install.log"; then
		echo "Install failed"
		exit 1
	fi

	# Throw an error install failed
	# (otherwise, build.sh will not stop)
	#if [[ $result != 0 ]]; then
	#	exit 1
	#fi
}



command="$1"
shift

if [ "$command" = install ] ; then
	install
elif [ "$command" = bash ] ; then
	bash $@
else
	echo "$0: unrecognized command \"$command\"."
	exit 1
fi
