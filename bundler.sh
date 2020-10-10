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
init-build        -- Initialize a Docker-based compilation of the TeXLive binaries.
make-installation -- Install TeXLive into a new directory tree.
make-base-zipfile -- Make a Zip file of a standardized base TeXLive installation.
run-build         -- Launch a Docker-based compilation of the TeXLive binaries.
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
    docker build -t $image_name:$tag bundler/
    docker tag $image_name:$tag $image_name:latest
}


function bundler_bash () {
    [ -d $state_dir/repo ] || die "no such directory $state_dir/repo"
    exec docker run -it --rm -v $state_dir:/state:rw,z $image_name bash
}


function init_build() {
    [ -d $state_dir/repo ] || die "no such directory $state_dir/repo"
    [ ! -d $state_dir/rbuild ] || die "directory $state_dir/rbuild may not exist before starting build"
    docker create \
           -v $state_dir:/state:rw,z \
           -i -t \
           --name $bundler_cont_name \
           $image_name bash || die "could not create bundler container $bundler_cont_name"
    docker start $bundler_cont_name || die "could not start bundler container $bundler_cont_name"
    exec docker exec $bundler_cont_name /entrypoint.sh init-build
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


function run_build() {
    [ -d $state_dir/repo ] || die "no such directory $state_dir/repo"
    [ -d $state_dir/rbuild ] || die "no such directory $state_dir/rbuild"
    docker start $bundler_cont_name || die "could not start bundler container $bundler_cont_name"
    echo "Building with logs to state/rbuild.log ..."
    docker exec $bundler_cont_name /entrypoint.sh bash -c 'cd /state/rbuild && make' &> state/rbuild.log \
           || die "build exited with an error code! consult the log file"
}


function update_containers () {
    [ -d $state_dir/repo ] || die "no such directory $state_dir/repo"
    mkdir -p $state_dir/containers $state_dir/versioned
    docker run --rm -v $state_dir:/state:rw,z $image_name update-containers

    # Make versioned copies of unmodified packages.

    cd "$state_dir"

    (cd containers/archive && ls -1) |while read cname ; do
	keep=false
	# TBD: are we going to need versioned packages of the binaries?
	case $cname in
	    *.doc.tar.xz | *.source.tar.xz | *.*-*.tar.xz | *.win32.tar.xz) ;;
	    *.tar.xz) keep=true ;;
	esac
	$keep || continue

	pkg=$(basename $cname .tar.xz)
	new=containers/archive/$cname
	tlp=tlpkg/tlpobj/$pkg.tlpobj
	rev=$(tar xf $new -O $tlp |grep ^revision |awk '{print $2}')
	versioned=versioned/$pkg-$rev.tar.xz

	if [ ! -f $versioned ] ; then
	    echo $pkg $rev
	    cp $new $versioned
	    chmod 444 $versioned
	fi
    done
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
    init-build)
        init_build "$@" ;;
    make-installation)
        make_installation "$@" ;;
    make-base-zipfile)
        make_base_zipfile "$@" ;;
    run-build)
        run_build "$@" ;;
    update-containers)
        update_containers "$@" ;;
    zip2itar)
        zip2itar "$@" ;;
    *)
        echo >&2 "error: unrecognized command \"$command\"."
        exit 1 ;;
esac
