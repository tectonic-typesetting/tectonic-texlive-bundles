#! /bin/bash
# Copyright 2016-2020 the Tectonic Project.
# Licensed under the MIT License.

image_name=tectonic-texlive-bundler
bundler_cont_name=tectonic-bld-cont
state_dir=$(pwd)/state # symlink here!

set -e

if [ -z "$1" -o "$1" = help ] ; then
    echo "You must supply a subcommand. Subcommands are:

build-image       -- Create or update the bundler Docker image.
bundler-bash      -- Run a shell in a temporary bundler container.
make-installation -- Install TeXLive into a new directory tree.
make-base-zipfile -- Make a Zip file of a standardized base TeXLive installation.
update-containers -- Rebuild the TeXLive \"container\" files.
zip2itar          -- Convert a bundle from Zip format to indexed-tar format.

"
    exit 1
fi

command="$1"
shift


function die () {
    echo >&2 "error:" "$@"
    exit 1
}


function build_image () {
    tag=$(date +%Y%m%d)
    docker build -t $image_name:$tag bundler-image/
    docker tag $image_name:$tag $image_name:latest
}


function bundler_bash () {
    [ -d $state_dir/repo ] || die "no such directory $state_dir/repo"
    exec docker run -it --rm -v $state_dir:/state:rw,z $image_name bash
}


function make_installation () {
    # arguments: names of TeXLive packages to install above and beyond the
    # "minimal" installation profile.

    [ -d $state_dir/repo ] || die "no such directory $state_dir/repo"

    dest=$(mktemp -d -p $state_dir install.XXXXXX)
    destbase=$(basename $dest)
    cdest=/state/$destbase

    # $cdest/texmf-dist is created and populated with most files.
    # TEXMFSYSCONFIG and TEXMFSYSVAR are filled with files that we might care about.
    # TEXMFLOCAL is created but doesn't have anything we care about.
    # TEXMFHOME, TEXMFCONFIG, and TEXMFVAR are not created.
    # option_sys_* are not created either.
    # Other settings are best guesses about what's sensible.

    cat >$dest/bundler.profile <<-EOF
    selected_scheme scheme-minimal
    TEXDIR $cdest
    TEXMFSYSCONFIG $cdest/texmf-dist
    TEXMFSYSVAR $cdest/texmf-dist
    TEXMFLOCAL $cdest/texmf-local
    TEXMFHOME $cdest/texmf-home
    TEXMFCONFIG $cdest/texmf-config
    TEXMFVAR $cdest/texmf-var
    collection-basic 1
    option_adjustrepo 1
    option_autobackup 0
    option_desktop_integration 0
    option_doc 0
    option_file_assocs 0
    option_fmt 1
    option_letter 1
    option_path 0
    option_post_code 1
    option_src 0
    option_sys_bin $cdest/sys-bin
    option_sys_info $cdest/sys-info
    option_sys_man $cdest/sys-man
    option_w32_multi_user 0
    option_write18_restricted 0
    portable 0
EOF
    echo $dest
    echo >&2 "Logging installation to $dest/outer.log ..."
    set +e
    docker run --rm -v $state_dir:/state:rw,z $image_name \
	   install-profile $cdest/bundler.profile $cdest $(id -u):$(id -g) "$@" &>$dest/outer.log
    ec=$?
    [ $ec -eq 0 ] || die "install-tl failed; see $dest/outer.log"
    set -e
}


function make_base_zipfile () {
    zip="$1"

    if [ -z "$zip" ] ; then
        die "usage: $0 make-base-zipfile <output-zip-filename>"
    fi

    bundle_id=tlextras2018
    shift

    # First, TeXLive package installation.

    installdir=$(make_installation \
         collection-basic \
         collection-bibtexextra \
         collection-fontsextra \
         collection-fontsrecommended \
         collection-humanities \
         collection-latexextra \
         collection-latexrecommended \
         collection-latex \
         collection-luatex \
         collection-mathscience \
         collection-music \
         collection-pictures \
         collection-plaingeneric \
         collection-publishers \
         collection-xetex \
         collection-langarabic \
         collection-langchinese \
         collection-langcjk \
         collection-langcyrillic \
         collection-langczechslovak \
         collection-langenglish \
         collection-langeuropean \
         collection-langfrench \
         collection-langgerman \
         collection-langgreek \
         collection-langitalian \
         collection-langjapanese \
         collection-langkorean \
         collection-langother \
         collection-langpolish \
         collection-langportuguese \
         collection-langspanish
    )

    # Some manual fiddles for format file generation

    cp extras/$bundle_id/* $installdir/texmf-dist/

    # Finally, turn it all into a Zip file.

    ./bundler/make-zipfile.py "$installdir" "$zip"
    rm -rf "$installdir"
}


function update_containers () {
    [ -d $state_dir/repo ] || die "no such directory $state_dir/repo"
    mkdir -p $state_dir/containers
    docker run --rm -v $state_dir:/state:rw,z $image_name update-containers
}


function zip2itar () {
    zipfile="$1"

    if [ ! -f "$zipfile" ] ; then
        die "no such input file \"$zipfile\""
    fi

    dir=$(cd $(dirname "$zipfile") && pwd)
    zipfull=$dir/$(basename "$zipfile")
    tarfull=$dir/$(basename "$zipfile" .zip).tar
    echo "Generating $tarfull ..."
    cd $(dirname $0)/zip2tarindex
    exec cargo run --release -- "$zipfull" "$tarfull"
}


# Dispatch subcommands.

case "$command" in
    build-image)
        build_image "$@" ;;
    bundler-bash)
        bundler_bash "$@" ;;
    make-installation)
        make_installation "$@" ;;
    make-base-zipfile)
        make_base_zipfile "$@" ;;
    update-containers)
        update_containers "$@" ;;
    zip2itar)
        zip2itar "$@" ;;
    *)
        echo >&2 "error: unrecognized command \"$command\"."
        exit 1 ;;
esac
