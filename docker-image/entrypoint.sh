#!/usr/bin/env bash
# Copyright 2016-2020 the Tectonic Project.
# Licensed under the MIT License.


function chown_host() {
    chown $HOSTUID:$HOSTGID -R "${1}"
}


# Install texlive into $1.
# Should be an absolute path.
function install () {
    source /bundle/bundle.sh
    local outpath="${1}/${bn_name}-${bn_texlive_version}"
    mkdir -p "${outpath}"

    # Load profile and update destination paths
    profile=$(mktemp)
    sed -e "s|@dest@|${outpath}|g" /bundle/tl-profile.txt > "${profile}"

    # Install texlive
    cd /iso
    ./install-tl -profile "${profile}" | tee "${outpath}/tl-install.log"
    
    # Cleanup
    rm "${profile}"
    chown_host "${outpath}"
}





command="$1"
shift

if [ "$command" = bash ] ; then
    exec bash "$@"
elif [ "$command" = python ] ; then
    exec python3 "$@"
elif [ "$command" = install ] ; then
    install "$@"
else
    echo "$0: unrecognized command \"$command\"."
    exit 1
fi
