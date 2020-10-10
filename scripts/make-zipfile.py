# -*- mode: python; coding: utf-8 -*-
# Copyright 2016-2018 the Tectonic Project.
# Licensed under the MIT License.

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
    bundle = Bundle.open_default()
    bundle.ensure_artfact_dir()

    paths = []

    try:
        zip_path = bundle.zip_path()
        paths.append(zip_path)

        with zipfile.ZipFile(zip_path, 'w', zipfile.ZIP_DEFLATED, True) as zip:
            b = ZipMaker(bundle, zip)
            b.go()

        print('Final SHA256SUM:', b.final_hexdigest)

        digest_path = bundle.digest_path()
        print(f'Creating digest file {cpath2qhpath(digest_path)}')
        paths.append(digest_path)

        with open(digest_path, 'wt') as f:
            print(b.final_hexdigest, file=f)

        listing_path = bundle.listing_path()
        print(f'Creating listing file {cpath2qhpath(listing_path)}')
        paths.append(listing_path)

        with open(listing_path, 'wt') as f:
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


if __name__ == '__main__':
    sys.exit(entrypoint(sys.argv))
