#! /usr/bin/env python3
# -*- mode: python; coding: utf-8 -*-
# Copyright 2020 the Tectonic Project.
# Licensed under the MIT License.

"""
Test generation of the formats defined in a bundle.
"""

import argparse
import os
import subprocess
import sys
import zipfile

from test_utils import *


def entrypoint(argv):
    settings = make_arg_parser().parse_args(argv[1:])
    bundle = Bundle.open_with_inferred_state(settings.bundle_dir)

    formatdir = bundle.test_path("formats")
    n_errors = 0
    n_tested = 0
    n_missing = 0
    n_removed = 0

    # Load the formats from the bundle

    bundle_formats = set()

    with open(bundle.listing_path()) as flist:
        for line in flist:
            base = line.strip()
            if base.startswith("tectonic-format-") and base.endswith(".tex"):
                bundle_formats.add(base[16:-4])

    # Compare to the test reference data

    ref_formats = set()

    with open(bundle.path("formats.txt")) as fref:
        for line in fref:
            ref_formats.add(line.strip())

    # Check that those lists agree

    for c in bundle_formats:
        if c not in ref_formats:
            print(f"MISSING {c} - not in formats.txt")
            n_missing += 1
            n_errors += 1

    for c in ref_formats:
        if c not in bundle_formats:
            print(f"REMOVED {c} - in formats.txt but not bundle")
            n_removed += 1
            n_errors += 1

    # Run the tests

    with zipfile.ZipFile(bundle.zip_path(), "r") as zip:
        for fmt in ref_formats:
            print(fmt, "... ", end="")
            n_tested += 1

            thisdir = os.path.join(formatdir, fmt)
            os.makedirs(thisdir, exist_ok=True)

            # Extract the format-creator from the bundle zip

            zip.extract(f"tectonic-format-{fmt}.tex", path=thisdir)

            # Run it

            with open(os.path.join(thisdir, "log.txt"), "wb") as log:
                result = subprocess.call(
                    [
                        TECTONIC_PROGRAM,
                        "-p",
                        "-b",
                        bundle.zip_path(),
                        "--outfmt",
                        "fmt",
                        os.path.join(thisdir, f"tectonic-format-{fmt}.tex"),
                    ],
                    shell=False,
                    stdout=log,
                    stderr=subprocess.STDOUT,
                )

            if result == 0:
                print("pass")
            else:
                print("FAIL")
                n_errors += 1

    print()
    print("Summary:")
    print(f"- Tested {n_tested} formats")
    if n_missing:
        print(f"- {n_missing} formats missing from formats.txt")
    if n_removed:
        print(f"- {n_removed} formats in formats.txt removed from bundle")
    if n_errors:
        print(f"- {n_errors} total errors: test failed (see outputs in {formatdir})")
    else:
        print(f"- no errors: test passed (outputs stored in {formatdir})")

    return 1 if n_errors else 0


def make_arg_parser():
    p = argparse.ArgumentParser()
    p.add_argument(
        "bundle_dir",
        help="The directory of the bundle specification",
    )
    return p


if __name__ == "__main__":
    sys.exit(entrypoint(sys.argv))
