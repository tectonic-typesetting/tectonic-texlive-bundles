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
    p.add_argument(
        'dest_path',
        metavar = 'PATH',
        help = 'The name of the Zip file to create.'
    )
    return p


def entrypoint(argv):
    settings = make_arg_parser().parse_args(argv[1:])
    bundle = Bundle.open_default()

    try:
        with zipfile.ZipFile(settings.dest_path, 'w', zipfile.ZIP_DEFLATED, True) as zip:
            b = ZipMaker(bundle, zip)
            b.go()
            print(b.final_hexdigest)
    except Exception as e:
        try:
            os.unlink(settings.dest_path)
        except:
            pass
        raise e


if __name__ == '__main__':
    sys.exit(entrypoint(sys.argv))
