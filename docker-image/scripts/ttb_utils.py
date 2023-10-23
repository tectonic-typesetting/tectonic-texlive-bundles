# -*- mode: python; coding: utf-8 -*-


"""
Utilities for the Tectonic bundler infrastructure.

Fixed characteristics of the environment:

- Source tree for the tools is in /source/
- Data/build directory is /build/
- Iso should be mounted at /iso/
- Bundle specification is in /bundle/
- The numeric UID and GID of the executing user in the host environment are
  stored in the environment variables $HOSTUID and $HOSTGID.

"""

__all__ = """
ZipMaker
chown_host
die
warn
""".split()

import contextlib
import hashlib
import os.path
import shutil
import struct
import subprocess
import sys
import tempfile


def warn(text):
    print("warning:", text, file=sys.stderr)


def die(text):
    raise SystemExit(f"error: {text}")


def chown_host(path, recursive=True):
    uid = int(os.environ["HOSTUID"])
    gid = int(os.environ["HOSTGID"])

    os.chown(path, uid, gid)

    if not recursive:
        return

    for dirpath, dirnames, filenames in os.walk(path):
        for dname in dirnames:
            os.lchown(os.path.join(dirpath, dname), uid, gid)

        for fname in filenames:
            os.lchown(os.path.join(dirpath, fname), uid, gid)



IGNORED_BASE_NAMES = set([
    "00readme.txt",
    "LICENSE.md",
    "Makefile",
    "README",
    "README.md",
    "ls-R",
])

IGNORED_EXTENSIONS = set([
    "fmt",
    "log",
    "lua",
    "mf",
    "pl",
    "ps",
])


class ZipMaker(object):
    def __init__(self, zip):
        self.zip = zip
        self.item_shas = {}
        self.final_hexdigest = None
        self.clashes = {}  # basename => {digest => fullpath}

        self.ignored_tex_paths = set()

        with open("/bundle/ignored-tex-paths.txt", "r") as f:
            for line in f:
                line = line.split("#")[0].strip()
                if len(line):
                    self.ignored_tex_paths.add(line)

        self.ignored_tex_path_prefixes = []

        with open("/bundle/ignored-tex-path-prefixes.txt", "r") as f:
            for line in f:
                line = line.split("#")[0].strip()
                if len(line):
                    self.ignored_tex_path_prefixes.append(line)


    def consider_file(self, tex_path, base_name):
        """
        Consider adding the specified TeXLive file to the installation tree.
        This is where all the nasty hairy logic will accumulate that enables us
        to come out with a nice pretty tarball in the end.
        """

        if base_name in IGNORED_BASE_NAMES:
            return False

        ext_bits = base_name.split(".", 1)
        if len(ext_bits) > 1 and ext_bits[1] in IGNORED_EXTENSIONS:
            return False

        if tex_path in self.ignored_tex_paths:
            return False

        for pfx in self.ignored_tex_path_prefixes:
            if tex_path.startswith(pfx):
                return False

        return True


    def _walk_onerr(self, oserror):
        warn(f"error navigating installation tree: {oserror}")


    # Actually building the full Zip

    def add_file(self, full_path):
        base = os.path.basename(full_path)

        # Get the digest

        with open(full_path, "rb") as f:
            contents = f.read()

        s = hashlib.sha256()
        s.update(contents)
        digest = s.digest()

        # OK, have we seen this before?

        prev_tuple = self.item_shas.get(base)

        if prev_tuple is None:
            # New basename, yay
            self.zip.writestr(base, contents)
            self.item_shas[base] = (digest, full_path)
        elif prev_tuple[0] != digest:
            # Already seen basename, and new contents :-(
            bydigest = self.clashes.setdefault(base, {})

            if not len(bydigest):
                # If this is the first duplicate, don't forget that we've seen
                # the file at least once before.
                bydigest[prev_tuple[0]] = [prev_tuple[1]]

            pathlist = bydigest.setdefault(digest, [])
            pathlist.append(full_path)


    def go(self):

        # Add the extra files preloaded in the bundle
        for name in os.listdir("/bundle/extras"):
            self.add_file(os.path.join("/bundle/extras", name))

        # Add the patched files, and make sure not to overwrite them later.
        patched_basenames = set()
        for name in os.listdir("/bundle/patched"):
            self.add_file(os.path.join("/bundle/patched", name))
            patched_basenames.add(name)

        # Add the main tree.

        p = os.path.join("/install", "texmf-dist")
        n = len(p) + 1
        print(f"Scanning {p} ...")

        for dirpath, _, filenames in os.walk(p, onerror=self._walk_onerr):
            for fn in filenames:
                if fn in patched_basenames:
                    continue

                full = os.path.join(dirpath, fn)
                tex = full[n:]
                if self.consider_file(tex, fn):
                    self.add_file(full)

        # Compute a hash of it all.

        print("Computing final hash ...")
        s = hashlib.sha256()
        s.update(struct.pack(">I", len(self.item_shas)))
        s.update(b"\0")

        for name in sorted(self.item_shas.keys()):
            s.update(name.encode("utf8"))
            s.update(b"\0")
            s.update(self.item_shas[name][0])

        self.final_hexdigest = s.hexdigest()
        self.zip.writestr("SHA256SUM", self.final_hexdigest)

        # Report clashes if needed

        if len(self.clashes):
            warn(f"{len(self.clashes)} clashing basenames were observed")

            report_path = "/output/clash-report.txt"
            warn(f"logging clash report to {report_path}")

            with open(report_path, "wt") as f:
                for base in sorted(self.clashes.keys()):
                    print(f"{base}:", file=f)
                    bydigest = self.clashes[base]

                    for digest in sorted(bydigest.keys()):
                        print(f"  {digest.hex()[:8]}:", file=f)

                        for full in sorted(bydigest[digest]):
                            print(f"     {full[n:]}", file=f)

            chown_host(report_path)


    def write_listing(self, stream):
        for base in sorted(self.item_shas.keys()):
            print(base, file=stream)