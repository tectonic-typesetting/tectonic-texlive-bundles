# -*- mode: python; coding: utf-8 -*-
# Copyright 2020 the Tectonic Project.
# Licensed under the MIT License.

"""
Utilities for the Tectonic bundler infrastructure.
"""

__all__ = '''
Bundle
chown_host
cpath2qhpath
die
warn
'''.split()

import contextlib
import os.path
import pytoml
import sys
import tempfile


def warn(text):
    print('warning:', text, file=sys.stderr)


def die(text):
    raise SystemExit(f'error: {text}')


def chown_host(path, recursive=True):
    uid = int(os.environ['HOSTUID'])
    gid = int(os.environ['HOSTGID'])

    os.chown(path, uid, gid)

    if not recursive:
        return

    for dirpath, dirnames, filenames in os.walk(path):
        for dname in dirnames:
            os.lchown(os.path.join(dirpath, dname), uid, gid)

        for fname in filenames:
            os.lchown(os.path.join(dirpath, fname), uid, gid)


def cpath2qhpath(container_path):
    "Container path to quoted host path."
    if container_path.startswith('/state/'):
        return f'`{container_path[1:]}`'

    return f'(container path) `{container_path}``'


class Bundle(object):
    cfg = None
    name = None
    version = None

    @classmethod
    def open_default(cls):
        inst = cls()

        with open('/bundle/bundle.toml', 'rt') as f:
            cfg = pytoml.load(f)

        inst.cfg = cfg
        inst.name = cfg['bundle']['name']
        inst.version = cfg['bundle']['version']

        return inst


    def path(self, *segments):
        return os.path.join('/bundle', *segments)


    def install_dir(self):
        return os.path.join('/state/installs', f'{self.name}-{self.version}')


    @contextlib.contextmanager
    def create_texlive_profile(self):
        dest = self.install_dir()

        with tempfile.NamedTemporaryFile(delete=False, mode='wt') as f:
            with open(self.path('tl-profile.txt'), 'rt') as template:
                for line in template:
                    line = line.replace('@dest@', dest)
                    print(line, file=f, end='')

            f.close()
            yield f.name
