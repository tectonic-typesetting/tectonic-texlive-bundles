# -*- mode: python; coding: utf-8 -*-
# Copyright 2020 the Tectonic Project.
# Licensed under the MIT License.

"""
This script is meant to be run inside the TeXLive bundler Docker container.

Create a TeXLive installation from a given bundle specification.
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

    # Validate that our current repo and the containers are in sync

    git_hash, svn_rev = get_repo_version()

    with open('/state/containers/GITHASH') as f:
        container_git_hash = f.readline().strip()

    if git_hash != container_git_hash:
        die(
            'refusing to proceed since current repo hash {git_hash} does not agree '
            'with that used to make containers; rerun `update-containers` step?'
        )

    # OK, good to go

    try:
        os.makedirs(install_dir)
    except Exception as e:
        raise Exception(f'cannot create bundle install directory with container path `{install_dir}`') from e

    chown_host('/state/installs', recursive=False)  # make sure this one is host-owned

    try:
        with open(os.path.join(install_dir, 'GITHASH'), 'wt') as f:
            print(git_hash, file=f)

        with open(os.path.join(install_dir, 'SVNREV'), 'wt') as f:
            print(svn_rev, file=f)

        log_path = os.path.join(install_dir, 'ttb-install.log')

        with bundle.create_texlive_profile() as profile_path:
            with open(log_path, 'wb') as log:
                print(f'Running install with logs to {cpath2qhpath(log_path)} ...')
                subprocess.check_call(
                    [
                        'Master/install-tl',
                        '--repository', '/state/containers',
                        '--profile', profile_path,
                    ],
                    shell = False,
                    stdout = log,
                    stderr = subprocess.STDOUT,
                    cwd = '/state/repo',
                )
    finally:
        chown_host(install_dir)


if __name__ == '__main__':
    sys.exit(entrypoint(sys.argv))
