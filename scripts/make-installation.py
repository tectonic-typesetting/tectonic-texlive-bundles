# -*- mode: python; coding: utf-8 -*-
# Copyright 2020 the Tectonic Project.
# Licensed under the MIT License.

"""
This script is meant to be run inside the TeXLive bundler Docker container.

Create a TeXLive installation from a given bundle specification.

Fixed characteristics of the environment:

- Source tree for the tools is in /source/
- Data/state directory is /state/
- TeXLive repository is in /state/repo/
- Bundle specification is in /bundle/

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
    install_dir = bundle.install_dir()

    try:
        os.makedirs(install_dir)
    except Exception as e:
        raise Exception(f'cannot create bundle install directory with container path `{install_dir}`') from e

    chown_host('/state/installs', recursive=False)  # make sure this one is host-owned

    try:
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
