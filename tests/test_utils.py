# -*- mode: python; coding: utf-8 -*-
# Copyright 2020 the Tectonic Project.
# Licensed under the MIT License.

"""
Utilities for testing Tectonic bundles, *outside* of the builder Docker
container.

Most of the Python code in this repo runs *inside* a Docker container to provide
a consistent environment for the TexLive scripts. But we want to be able to test
bundles using whatever Tectonic executable happens to be available, so the tests
run *outside* of Docker. For the moment I'm just duplicating the code rather
than devising some system that can transparently run in either situation. It's
gross.

"""

__all__ = '''
TECTONIC_PROGRAM
Bundle
die
warn
'''.split()

import os.path
import sys

import toml


TECTONIC_PROGRAM = os.environ.get('TECTONIC', 'tectonic')


def warn(text):
    print('warning:', text, file=sys.stderr)


def die(text):
    raise SystemExit(f'error: {text}')


class Bundle(object):
    statedir = None
    basedir = None
    cfg = None
    name = None
    version = None

    def __init__(self, statedir, basedir):
        self.statedir = statedir
        self.basedir = basedir

        with open(self.path('bundle.toml'), 'rt') as f:
            cfg = toml.load(f)

        self.cfg = cfg
        self.name = cfg['bundle']['name']
        self.version = cfg['bundle']['version']


    @classmethod
    def open_with_inferred_state(cls, basedir):
        top_dir = os.path.dirname(os.path.dirname(__file__))
        statedir = os.path.join(top_dir, 'state')
        return cls(statedir, basedir)


    def path(self, *segments):
        return os.path.join(self.basedir, *segments)


    def test_path(self, *segments):
        return os.path.join(self.statedir, 'testdata', f'{self.name}-{self.version}', *segments)


    def artifact_path(self, *segments):
        return os.path.join(self.statedir, 'artifacts', f'{self.name}-{self.version}', *segments)


    def zip_path(self):
        return self.artifact_path(f'{self.name}-{self.version}.zip')


    def listing_path(self):
        return self.artifact_path(f'{self.name}-{self.version}.listing.txt')
