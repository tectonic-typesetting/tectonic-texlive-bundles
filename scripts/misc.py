# -*- mode: python; coding: utf-8 -*-
# Copyright 2016-2018 the Tectonic Project.
# Licensed under the MIT License.

"""
This script is meant to be run inside the TeXLive bundler Docker container.

Some miscellaneous operations
"""

import sys

from ttb_utils import *


def entrypoint(argv):
    if argv[1] == 'zip-relpath':
        b = Bundle.open_default()
        # sketchy ...
        print(cpath2qhpath(b.zip_path())[1:-1])
    else:
        die('unknown util.py subcommand')

    return 0

if __name__ == '__main__':
    sys.exit(entrypoint(sys.argv))
