# -*- mode: python; coding: utf-8 -*-

"""
This script is meant to be run inside the TeXLive bundler Docker container.

Create a Zip file containing all of the resources from a TeXLive
installation.
"""

import argparse
import os.path
import sys
import zipfile

from ttb_utils import *


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
        zip_path = f"/output/{name}-{version}.zip"
        paths.append(zip_path)

        with zipfile.ZipFile(zip_path, "w", zipfile.ZIP_DEFLATED, True) as zip:
            b = ZipMaker(zip)
            b.go()

        print("Final SHA256SUM:", b.final_hexdigest)

        digest_path = f"/output/{name}-{version}.sha256sum"
        print(f"Creating digest file in {digest_path}")
        paths.append(digest_path)

        with open(digest_path, "wt") as f:
            print(b.final_hexdigest, file=f)

        listing_path = f"/output/{name}-{version}.listing.txt"
        print(f"Creating listing file in {listing_path}")
        paths.append(listing_path)

        with open(listing_path, "wt") as f:
            b.write_listing(f)

        for p in paths:
            chown_host(p, recursive=False)
    except Exception as e:
        try:
            for p in paths:
                os.unlink(p)
        except:
            pass
        raise e


if __name__ == "__main__":
    sys.exit(entrypoint(sys.argv))
