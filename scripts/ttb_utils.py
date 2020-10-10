# -*- mode: python; coding: utf-8 -*-
# Copyright 2020 the Tectonic Project.
# Licensed under the MIT License.

"""
Utilities for the Tectonic bundler infrastructure.

Fixed characteristics of the environment:

- Source tree for the tools is in /source/
- Data/state directory is /state/
- TeXLive repository is in /state/repo/
- Bundle specification is in /bundle/
- The numeric UID and GID of the executing user in the host environment are
  stored in the environment variables $HOSTUID and $HOSTGID.

"""

__all__ = '''
Bundle
ZipMaker
chown_host
cpath2qhpath
die
warn
'''.split()

import contextlib
import hashlib
import os.path
import pytoml
import struct
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


ignored_base_names = set([
    'LICENSE.md',
    'Makefile',
    'README',
    'README.md',
    'ls-R',
])

ignored_tex_paths = set([
    'tex/generic/tex-ini-files/pdftexconfig.tex',
    'tex/luatex/hyph-utf8/etex.src',
])


class ZipMaker(object):
    def __init__(self, bundle, zip):
        self.bundle = bundle
        self.zip = zip
        self.item_shas = {}
        self.final_hexdigest = None
        self.clashes = {}  # basename => {digest => fullpath}


    def add_file(self, full_path):
        base = os.path.basename(full_path)

        # Even if the basename has already been seen, we want to read in the
        # file to compare digests.

        with open(full_path, 'rb') as f:
            contents = f.read()

        s = hashlib.sha256()
        s.update(contents)
        digest = s.digest()

        prev = self.item_shas.get(base)

        if prev is None:
            # New basename, yay
            self.zip.writestr(base, contents)
            self.item_shas[base] = digest
        elif prev != digest:
            # Already seen basename, and new contents :-(
            bydigest = self.clashes.setdefault(base, {})

            if not len(bydigest):
                # If this is the first duplicate, we should mark that we've seen
                # the file at least once before. We don't know the full path
                # where it came from, but we have the digest.
                bydigest[prev] = ['(elsewhere)']

            pathlist = bydigest.setdefault(digest, [])
            pathlist.append(full_path)


    def consider_file(self, full_path, tex_path, base_name):
        """
        Consider adding the specified TeXLive file to the installation tree.
        This is where all the nasty hairy logic will accumulate that enables us
        to come out with a nice pretty tarball in the end.
        """

        if base_name in ignored_base_names:
            return

        if tex_path in ignored_tex_paths:
            return

        if base_name.endswith('.log'):
            return

        if base_name.endswith('.fmt'):
            return

        self.add_file(full_path)


    def _walk_onerr(self, oserror):
        warn(f'error navigating installation tree: {oserror}')


    def go(self):
        install_dir = self.bundle.install_dir()

        # Add a couple of version files from the builder.

        p = os.path.join(install_dir, 'SVNREV')
        if os.path.exists(p):
            self.add_file(p)
        else:
            warn(f'expected but did not see the file `{p}`')

        p = os.path.join(install_dir, 'GITHASH')
        if os.path.exists(p):
            self.add_file(p)
        else:
            warn(f'expected but did not see the file `{p}`')

        # Add the main tree.

        p = os.path.join(install_dir, 'texmf-dist')
        n = len(p) + 1

        for dirpath, dirnames, filenames in os.walk(p, onerror=self._walk_onerr):
            for fn in filenames:
                full = os.path.join(dirpath, fn)
                tex = full[n:]
                self.consider_file(full, tex, fn)

        # Compute a hash of it all.

        s = hashlib.sha256()
        s.update(struct.pack('>I', len(self.item_shas)))
        s.update(b'\0')

        for name in sorted(self.item_shas.keys()):
            s.update(name.encode('utf8'))
            s.update(b'\0')
            s.update(self.item_shas[name])

        self.final_hexdigest = s.hexdigest()
        self.zip.writestr('SHA256SUM', self.final_hexdigest)

        # Report clashes

        if len(self.clashes):
            warn('clashing basenames were observed:')
            print('', file=sys.stderr)

            for base in sorted(self.clashes.keys()):
                print(f'  {base}:', file=sys.stderr)
                bydigest = self.clashes[base]

                for digest in sorted(bydigest.keys()):
                    print(f'    {digest.hex()}:', file=sys.stderr)

                    for full in sorted(bydigest[digest]):
                        print(f'       {full[n:]}', file=sys.stderr)
