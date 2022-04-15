# -*- mode: python; coding: utf-8 -*-
# Copyright 2016-2022 the Tectonic Project.
# Licensed under the MIT License.

"""
This script is meant to be run inside the TeXLive bundler Docker container.

Extract copies of "vendor pristine" files that have patches maintained against
them. This helps us use Git branches to maintain the patches.
"""

import argparse
import sys

from ttb_utils import *


def make_arg_parser():
    p = argparse.ArgumentParser(
        description = __doc__,
    )
    return p


def entrypoint(argv):
    _settings = make_arg_parser().parse_args(argv[1:])
    bundle = Bundle.open_default()
    bundle.ensure_artfact_dir()
    maker = ZipMaker(bundle, None)
    maker.extract_vendor_pristine()


if __name__ == '__main__':
    sys.exit(entrypoint(sys.argv))
