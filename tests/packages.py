#! /usr/bin/env python3
# -*- mode: python; coding: utf-8 -*-
# Copyright 2020-2021 the Tectonic Project.
# Licensed under the MIT License.

"""
Test builds using some of the LaTeX package (style) files provided in a bundle.

There are thousands of these (about 5000 as of TeXLive 2020), so we use a
reproducible-random scheme to skip most of them to keep the testing time
reasonable. In particular:

- I did an initial run over all of the packages on the TeXLive 2020 bundle when
  setting this all up. All of the packages that failed were marked with a "skip"
  tag. These are always skipped.

- All of the packages were assigned a randomly-generated number between 0 and 99
  (inclusive), using a `rand=` key in the listing file. Of the remaining
  non-"skip" packages, only a fraction of them are tested, using the random key
  to select them. This program takes a `-S` option to specify the percentage of
  packages to test, and a `-K` option to specify which random subset to
  investigate. Packages where `(randkey + K) % 100 >= S` are skipped.

- Packages without a `rand=` setting are always tested.

- The default `-S` setting is 5%, which tests about 150 packages and takes about
  7 minutes to run. The default `-K` setting is random.

"""

import argparse
import os.path
import random
import subprocess
import sys

from test_utils import *

# We use percent formatting since all the TeX braces would be super annoying to
# escape in str.format() formatting.
DOC_CLASS_TEMPLATE = r"\documentclass{%(class)s}"
PACKAGE_TEMPLATE = r"\usepackage{%(package)s}"

DOCUMENT_BODY = r"""\begin{document}
Hello, world.
\end{document}"""


def entrypoint(argv):
    settings = make_arg_parser().parse_args(argv[1:])
    bundle = Bundle.open_with_inferred_state(settings.bundle_dir)

    packagedir = bundle.test_path("packages")
    n_errors = 0
    n_surprises = 0
    n_tested = 0
    n_skipped = 0
    n_missing = 0
    n_removed = 0
    n_xfail = 0

    # Random sampling setup

    if settings.sample_key is None:
        settings.sample_key = random.randint(0, 99)

    if settings.bootstrap:
        print(
            "note: bootstrap mode engaged! Testing *all* packages and rewriting packages.txt"
        )
        print()

    # Load the packages from the bundle

    bundle_packages = set()

    with open(bundle.listing_path()) as flist:
        for line in flist:
            base = line.strip()
            if base.endswith(".sty"):
                bundle_packages.add(base[:-4])

    # Compare to the test reference data, if we're not in bootstrap mode:

    ref_packages = {}

    if settings.bootstrap:
        for pkg in bundle_packages:
            ref_packages[pkg] = {
                "tags": ["ok"],
                "randkey": random.randint(0, 99),
            }

        # Minimize churn in randkeys if there's already an input file
        packages_path = bundle.path("packages.txt")

        if os.path.exists(packages_path):
            with open(packages_path) as fref:
                for line in fref:
                    bits = line.split()
                    classname = bits[0]

                    if classname not in ref_packages:
                        continue

                    saw_rand = False

                    for bit in bits[2:]:
                        if bit.startswith("rand="):
                            ref_packages[classname]["randkey"] = int(bit[5:])
                            saw_rand = True
                            break

                    if (
                        not saw_rand
                    ):  # Preserve items with no randkey -- these are always tested
                        del ref_packages[classname]["randkey"]
    else:
        with open(bundle.path("packages.txt")) as fref:
            for line in fref:
                bits = line.split()
                classname = bits[0]
                info = {}

                info["tags"] = bits[1].split(",")

                for bit in bits[2:]:
                    if bit.startswith("rand="):
                        info["randkey"] = int(bit[5:])
                    else:
                        die(f"unexpected metadata item {bit!r} in packages.txt")

                ref_packages[classname] = info

        # Check that the lists agree

        for p in bundle_packages:
            if p not in ref_packages:
                print(f"MISSING {p} - not in packages.txt")
                n_missing += 1
                n_errors += 1
                bootstrap_text = ""

        for p in ref_packages.keys():
            if p not in bundle_packages:
                print(f"REMOVED {p} - in packages.txt but not bundle")
                n_removed += 1
                n_errors += 1
                bootstrap_text = ""

        if n_missing + n_removed > 0:
            print("NOTE: use --bootstrap to rebuild packages.txt if needed")

    # Sampling setup.

    if settings.bootstrap:
        settings.sample_percentage = 100  # not used, but for tidyness' sake
    else:
        if settings.sample_percentage is None:
            TARGET_N_PACKAGES = 100
            settings.sample_percentage = max(
                100 * TARGET_N_PACKAGES // len(ref_packages), 1
            )
            n_eff = settings.sample_percentage * len(ref_packages) // 100
            print(
                f"note: targeting about {n_eff} randomized test cases ({settings.sample_percentage}% of corpus; actual number will vary)"
            )
        else:
            print(
                f"note: sampling {settings.sample_percentage}% of the randomized test cases"
            )

        print(
            f"note: sample key is {settings.sample_key}; use argument `-K {settings.sample_key}` to reproduce this run`"
        )

    # Run the tests

    if settings.bootstrap:
        pkg_file = open(bundle.path("packages.txt"), "wt")
        pkg_iter = sorted(ref_packages.keys())
    else:
        pkg_file = None
        pkg_iter = ref_packages.keys()

    for pkg in pkg_iter:
        info = ref_packages[pkg]
        tags = info["tags"]

        if "randkey" in info and not settings.bootstrap:
            effkey = (info["randkey"] + settings.sample_key) % 100
            random_skipped = effkey >= settings.sample_percentage
        else:
            random_skipped = False

        if "skip" in tags or random_skipped:
            n_skipped += 1
            continue

        print(pkg, "... ", end="")
        sys.stdout.flush()
        n_tested += 1

        thisdir = os.path.join(packagedir, pkg)
        os.makedirs(thisdir, exist_ok=True)

        texpath = os.path.join(thisdir, "index.tex")

        params = {
            "class": "article",
            "package": pkg,
        }

        with open(texpath, "wt") as f:
            print(DOC_CLASS_TEMPLATE % params, file=f)
            print(PACKAGE_TEMPLATE % params, file=f)
            print(DOCUMENT_BODY, file=f)

        with open(os.path.join(thisdir, "log.txt"), "wb") as log:
            result = subprocess.call(
                [TECTONIC_PROGRAM, "-p", "-b", bundle.zip_path(), texpath],
                shell=False,
                stdout=log,
                stderr=subprocess.STDOUT,
            )

        if result == 0:
            if "ok" in tags:
                print("pass", flush=True)
            else:
                # This test succeeded even though we didn't expect it to.
                # Not a bad thing, but worth noting!
                print("pass (unexpected)", flush=True)
                n_surprises += 1
        else:
            if "xfail" in tags:
                print("xfail", flush=True)
                n_xfail += 1
            else:
                # This test failed unexpectedly :-(
                print("FAIL", flush=True)
                n_errors += 1

        if settings.bootstrap:
            tag = "ok" if result == 0 else "skip"
            print(f'{pkg} {tag} rand={info["randkey"]}', file=pkg_file)

    print()
    print("Summary:")
    print(f"- Tested {n_tested} packages")
    if n_skipped:
        print(f"- {n_skipped} cases skipped")
    if n_missing:
        print(f"- {n_missing} packages missing from packages.txt")
    if n_removed:
        print(f"- {n_removed} packages in packages.txt removed from bundle")
    if n_xfail:
        print(f"- {n_xfail} expected failures")
    if n_surprises:
        print(f"- {n_surprises} surprise passes")
    if n_errors:
        if settings.bootstrap:
            print(f"- {n_errors} total build failures (see outputs in {packagedir})")
        else:
            print(
                f"- {n_errors} total errors: test failed (outputs stored in {packagedir})"
            )
    else:
        print(f"- no errors: test passed (outputs stored in {packagedir})")

    if settings.bootstrap:
        pkg_file.close()

    return 1 if n_errors and not settings.bootstrap else 0


def make_arg_parser():
    p = argparse.ArgumentParser()
    p.add_argument(
        "--bootstrap",
        action="store_true",
        help="Bootstrap mode: test all packages, rewrite packages.txt",
    )
    p.add_argument(
        "-S",
        "--samp-pct",
        dest="sample_percentage",
        type=int,
        help="The percentage of test cases to sample",
    )
    p.add_argument(
        "-K",
        "--samp-key",
        dest="sample_key",
        type=int,
        help='The "key" determining which random subset of cases are sampled',
    )
    p.add_argument(
        "bundle_dir",
        help="The directory of the bundle specification",
    )
    return p


if __name__ == "__main__":
    sys.exit(entrypoint(sys.argv))
