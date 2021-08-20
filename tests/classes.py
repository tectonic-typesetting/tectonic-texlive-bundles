#! /usr/bin/env python3
# -*- mode: python; coding: utf-8 -*-
# Copyright 2020 the Tectonic Project.
# Licensed under the MIT License.

"""
Test builds using all of the LaTeX documentclasses provided in a bundle.
"""

import argparse
import os
import subprocess
import sys

from test_utils import *

# We use percent formatting since all the TeX braces would be super annoying to
# escape in str.format() formatting.
DOC_CLASS_TEMPLATE = r'\documentclass{%(class)s}'

TITLE_AUTHOR = r'''\title{Test Title}
\author{An Author}'''

DOCUMENT_BODY = r'''\begin{document}
Hello, world.
\end{document}'''


def entrypoint(argv):
    settings = make_arg_parser().parse_args(argv[1:])
    bundle = Bundle.open_with_inferred_state(settings.bundle_dir)

    classdir = bundle.test_path('classes')
    n_errors = 0
    n_surprises = 0
    n_tested = 0
    n_missing = 0
    n_removed = 0
    n_xfail = 0

    # Load the classes from the bundle

    bundle_classes = set()

    with open(bundle.listing_path()) as flist:
        for line in flist:
            base = line.strip()
            if base.endswith('.cls'):
                bundle_classes.add(base[:-4])

    # Compare to the test reference data

    ref_classes = {}

    with open(bundle.path('classes.txt')) as fref:
        for line in fref:
            bits = line.split()
            classname = bits[0]
            tags = bits[1].split(',')
            ref_classes[classname] = tags

    # Check that those lists agree

    for c in bundle_classes:
        if c not in ref_classes:
            print(f'MISSING {c} - not in classes.txt')
            n_missing += 1
            n_errors += 1

    for c in ref_classes.keys():
        if c not in bundle_classes:
            print(f'REMOVED {c} - in classes.txt but not bundle')
            n_removed += 1
            n_errors += 1

    # Run the tests

    for cls, flags in ref_classes.items():
        print(cls, '... ', end='')
        n_tested += 1

        thisdir = os.path.join(classdir, cls)
        os.makedirs(thisdir, exist_ok=True)

        texpath = os.path.join(thisdir, 'index.tex')

        params = {
            'class': cls,
        }

        with open(texpath, 'wt') as f:
            print(DOC_CLASS_TEMPLATE % params, file=f)

            if 'titleauth' in flags:
                print(TITLE_AUTHOR, file=f)

            print(DOCUMENT_BODY, file=f)

        with open(os.path.join(thisdir, 'log.txt'), 'wb') as log:
            result = subprocess.call(
                [TECTONIC_PROGRAM, '-p', '-b', bundle.zip_path(), texpath],
                shell = False,
                stdout = log,
                stderr = subprocess.STDOUT,
            )

        if result == 0:
            if 'ok' in flags:
                print('pass')
            else:
                # This test succeeded even though we didn't expect it to.
                # Not a bad thing, but worth noting!
                print('pass (unexpected)')
                n_surprises += 1
        else:
            if 'xfail' in flags:
                print('xfail')
                n_xfail += 1
            else:
                # This test failed unexpectedly :-(
                print('FAIL')
                n_errors += 1

    print()
    print('Summary:')
    print(f'- Tested {n_tested} documentclasses')
    if n_missing:
        print(f'- {n_missing} documentclasses missing from classes.txt')
    if n_removed:
        print(f'- {n_removed} documentclasses in classes.txt removed from bundle')
    if n_xfail:
        print(f'- {n_xfail} expected failures')
    if n_surprises:
        print(f'- {n_surprises} surprise passes')
    if n_errors:
        print(f'- {n_errors} total errors: test failed (see outputs in {classdir})')
    else:
        print(f'- no errors: test passed (outputs stored in {classdir})')

    return 1 if n_errors else 0


def make_arg_parser():
    p = argparse.ArgumentParser()
    p.add_argument(
        'bundle_dir',
        help = 'The directory of the bundle specification',
    )
    return p


if __name__ == '__main__':
    sys.exit(entrypoint(sys.argv))
