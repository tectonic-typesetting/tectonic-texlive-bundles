# -*- mode: python; coding: utf-8 -*-

"""
This script is meant to be run inside the TeXLive bundler Docker container.

Create a Zip file containing all of the resources from a TeXLive
installation.
"""

import argparse
import sys
import os
import zipfile
import hashlib
import struct
from pathlib import Path


# Bundle parameters
NAME = os.environ["bn_name"]
VERSION = os.environ["bn_texlive_version"]

# Input paths
PATH_ignore  = Path("/bundle/ignore")
PATH_extras  = Path("/bundle/extras")
PATH_patched = Path("/bundle/patched")
PATH_texlive = Path("/install/texmf-dist")

# Output paths
PATH_clash   = Path("/output/clash-report.txt")
PATH_zip     = Path(f"/output/{NAME}-{VERSION}.zip")
PATH_hash    = Path(f"/output/{NAME}-{VERSION}.sha256sum")
PATH_listing = Path(f"/output/{NAME}-{VERSION}.listing.txt")



class ZipMaker(object):
    def __init__(self, zip):
        self.zip = zip
        self.item_shas = {}
        self.final_hexdigest = None

        # Keeps track of conflicting file names
        # { "basename": {
        #       b"digest": Path(fullpath)
        # }}
        self.clashes = {}


        # Load ignore patterns
        self.ignore_patterns = set()
        if PATH_ignore.is_file():
            with PATH_ignore.open("r") as f:
                for line in f:
                    line = line.split("#")[0].strip()
                    if len(line):
                        self.ignore_patterns.add(line)


    def consider_file(self, file):
        """
        Consider adding the specified TeXLive file to the installation tree.
        This is where all the nasty hairy logic will accumulate that enables us
        to come out with a nice pretty tarball in the end.
        """

        for pattern in self.ignore_patterns:
            if file.relative_to(PATH_texlive).match(pattern):
                return False

        return True


    def add_file(self, full_path: Path):
        # Get the digest
        with open(full_path, "rb") as f:
            contents = f.read()

        s = hashlib.sha256()
        s.update(contents)
        digest = s.digest()

        # Have we seen this filename before?
        prev_tuple = self.item_shas.get(full_path.name)
        if prev_tuple is None:
            # This is a new file, ok for now.
            self.zip.writestr(full_path.name, contents)
            self.item_shas[full_path.name] = (digest, full_path)
        elif prev_tuple[0] != digest:
            # We already have a file with this name and different contents
            bydigest = self.clashes.setdefault(full_path.name, {})

            if not len(bydigest):
                # If this is the first duplicate, don't forget that we've seen
                # the file at least once before.
                bydigest[prev_tuple[0]] = [prev_tuple[1]]

            pathlist = bydigest.setdefault(digest, [])
            pathlist.append(full_path)




    def go(self):
        # Add the extra files preloaded in the bundle
        for f in PATH_extras.iterdir():
            self.add_file(f)

        # Add the patched files, and make sure not to overwrite them later.
        patched_basenames = set()
        for f in PATH_patched.iterdir():
            self.add_file(f)
            patched_basenames.add(f.name)

        # Add the main tree.
        ignored = 0
        print(f"Zipping {PATH_texlive}...")
        for f in PATH_texlive.rglob("*"):
            if not f.is_file():
                continue
            if f.name in patched_basenames:
                continue

            if self.consider_file(f):
                self.add_file(f)
            else:
                ignored += 1

        print(f"Done, ignored {ignored} files.")

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
            print(f"WARNING: {len(self.clashes)} file clashes were found.")
            print(f"Logging clash report to {PATH_clash}")

            with PATH_clash.open("w") as f:
                for filename in sorted(self.clashes.keys()):
                    f.write(f"{filename}:\n")
                    bydigest = self.clashes[filename]

                    for digest in sorted(bydigest.keys()):
                        f.write(f"\t{digest.hex()[:8]}:\n")

                        for path in sorted(bydigest[digest]):
                            f.write(f"\t\t{path}\n")
                    f.write("\n\n")


    def write_listing(self, file):
        for base in sorted(self.item_shas.keys()):
            file.write(base+"\n")


def make_arg_parser():
    p = argparse.ArgumentParser(
        description = __doc__,
    )
    return p


def entrypoint(argv):
    settings = make_arg_parser().parse_args(argv[1:])

    paths = []

    name = os.environ["bn_name"]
    version = os.environ["bn_texlive_version"]

    try:
        paths.append(PATH_zip)

        with zipfile.ZipFile(PATH_zip, "w", zipfile.ZIP_DEFLATED, True) as zip:
            b = ZipMaker(zip)
            b.go()

        print("Final SHA256SUM:", b.final_hexdigest)

        print(f"Creating digest file in {PATH_hash}")
        paths.append(PATH_hash)
        with PATH_hash.open("w") as f:
            f.write(b.final_hexdigest+"\n")

        print(f"Creating listing file in {PATH_listing}")
        paths.append(PATH_listing)
        with PATH_listing.open("w") as f:
            b.write_listing(f)

    except Exception as e:
        try:
            for p in paths:
                os.unlink(p)
        except:
            pass
        raise e


if __name__ == "__main__":
    sys.exit(entrypoint(sys.argv))
