# -*- mode: python; coding: utf-8 -*-
# Copyright 2020 the Tectonic Project.
# Licensed under the MIT License.

"""
This script is meant to be run inside the TeXLive bundler Docker container.

Install the specified packages into a given TeXLive installation.

"""

import argparse
import os
import subprocess
import sys

from ttb_utils import *


def make_arg_parser():
    p = argparse.ArgumentParser()
    return p


def entrypoint(argv):
    settings = make_arg_parser().parse_args(argv[1:])
    bundle = Bundle.open_default()
    install_dir = bundle.install_path()

    with open(bundle.path('packages.txt')) as f:
        packages = [l.strip() for l in f]

    # Validate that current repo, containers, and install are in sync

    git_hash, _ = get_repo_version()

    with open('/state/containers/GITHASH') as f:
        container_git_hash = f.readline().strip()

    if git_hash != container_git_hash:
        die(
            'refusing to proceed since current repo hash {git_hash} does not agree '
            'with that used to make containers; rerun `update-containers` step?'
        )

    with open(os.path.join(install_dir, 'GITHASH')) as f:
        install_git_hash = f.readline().strip()

    if git_hash != install_git_hash:
        die(
            'refusing to proceed since current repo hash {git_hash} does not agree '
            'with that used to make the installtion; recreate it?'
        )

    # OK, looks good

    try:
        # Note: the leading `./` in the exe path is vital so that the Perl code
        # can figure out its module search path.
        args = [
            './bin/x86_64-linux/tlmgr',
            '--repository', '/state/containers',
            'install',
        ]
        args += packages

        subprocess.check_call(
            args,
            shell = False,
            cwd = install_dir,
        )
    finally:
        chown_host(install_dir)


if __name__ == '__main__':
    sys.exit(entrypoint(sys.argv))
