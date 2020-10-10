#! /bin/bash
# Copyright 2016-2020 the Tectonic Project.
# Licensed under the MIT License.
#
# This is the "entrypoint" script for the TeXLive bundler image. When a user runs
# "docker run texlive-bundler foo bar", we are invoked with $1=foo and $2=bar.

set -e

if [ -z "$1" -o "$1" = help ] ; then
    echo "You must supply a subcommand to run in the container. Commands are:

bash              -- Run a bash shell
python            -- Run a Python script
update-containers -- Rebuild the TeXLive \"container\" files

Use 'docker run --rm -it bundlercontainer bash' to launch an interactive
shell within a long-running container.
"
    exit 1
fi

command="$1"
shift


function _precise_version () {
    # Log the precise version of the TeXLive tree that we are using. This is
    # done by making files named GITHASH and SVNREV in a directory named in
    # $1. We exit with an error if the tree is not pristine. This must be run
    # from a directory inside the Git repository.

    destdir="$1"

    if ! git update-index -q --refresh ; then
        echo >&2 "error: git update-index failed"
        exit 1
    fi

    if [ -n "$(git diff-index --name-only HEAD --)" ] ; then
        echo >&2 "error: refusing to make packages from dirty working tree"
        exit 1
    fi

    git show-ref --head |head -n1 |awk '{print $1}' >"$destdir/GITHASH"
    git show -s |grep git-svn-id |sed -e 's/.*@//' -e 's/ .*//' >"$destdir/SVNREV"
}

function update_containers () {
    cd /state/repo/
    _precise_version /state/containers

    args=(--all --gpgcmd=true --location /state/containers)
    if [ ! -e /state/containers/tlpkg/texlive.tlpdb ] ; then
        args+=(--recreate)
    fi

    exec Master/tlpkg/bin/tl-update-containers "${args[@]}"
}


# Dispatch subcommands.

if [ "$command" = bash ] ; then
    exec bash "$@"
elif [ "$command" = python ] ; then
    exec python3 "$@"
elif [ "$command" = update-containers ] ; then
    update_containers "$@"
else
    echo "$0: unrecognized command \"$command\"."
    exit 1
fi
