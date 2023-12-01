# -*- mode: python; coding: utf-8 -*-

"""
This script creates a tectonic zip bundle using a finished
texlive install and a bundle specification.
"""


# Tested with Python 3.11.5
#
# You shouldn't need a venv,
# these are all in stdlib
import sys
import zipfile
import hashlib
import struct
from pathlib import Path
import subprocess
import re
import shutil


# Bundle parameters
PATH_bundle = Path(sys.argv[1])


def get_var(varname):
    bundle_meta = PATH_bundle / "bundle.sh"
    p = subprocess.Popen(
        f"echo $(source {bundle_meta}; echo ${varname})",
        stdout=subprocess.PIPE,
        shell=True,
        executable="bash"
    )
    return p.stdout.readlines()[0].strip().decode("utf-8")

VAR_bundlename = get_var('bundle_name')
VAR_texlive = get_var('bundle_texlive_name')


# Input paths
PATH_ignore  = PATH_bundle / "ignore"
PATH_extra   = PATH_bundle / "include"
PATH_texlive = Path(f"build/texlive/{VAR_texlive}")

# Output paths
PATH_output  = Path(f"build/output/{VAR_bundlename}")
PATH_content = PATH_output / "content"



# Given a search spec and a list of paths, try to pick one path.
# This is a direct copy of the Rust implementation.
#
# We don't actually pick a path here, we just decide whether or not we
# *can*, given the current rules.
def search_for_file(search, paths):
    resolved = False
    for rule in search:
        for path in paths:
            if rule.endswith("//"):
                if str(path).startswith(rule[:-1]):
                    if resolved:
                        return False
                    else:
                        resolved = True
            else:
                if path.parent == Path(rule):
                    if resolved:
                        return False
                    else:
                        resolved = True

        if resolved:
            break

    return resolved


class FilePicker(object):
    def __init__(self):

        # Statistics
        self.extra_count = 0 # Extra files added
        self.extra_conflict_count = 0 # Number of conflicting extra files (0, ideally)
        self.added_count = 0 # Number of files from texlive
        self.ignored_count = 0 # Number of texlive files ignored
        self.replaced_count = 0 # Number of texlive files replaced with extra files
        self.patch_applied_count = 0 # How many diffs we've applied


        # Dict of { "filename": [Path] }
        # We may have repeating file names.
        self.index = {}

        # Dict of ( Path: hash )
        self.item_shas = {}

        # All filenames added from include dir
        self.extra_basenames = set()

        # Array of diff file paths in include dir.
        # Scanned at start of run, applied while running.
        # Map of "filename": Path(filename.diff)
        self.diffs = {}

        # Length of "Patching (n)" string,
        # used only for pretty printing.
        self.print_len = 0

        # Load ignore patterns
        self.ignore_patterns = set()
        if PATH_ignore.is_file():
            with PATH_ignore.open("r") as f:
                for line in f:
                    line = line.split("#")[0].strip()
                    if len(line):
                        self.ignore_patterns.add(line)


    # Print and pad with spaces.
    # Used when printing info while adding files.
    def clearprint(self, string):
        l = len(string)
        if l < self.print_len:
            string += " "*l
        print(string)


    # Returns true if we should add this file to our bundle.
    def consider_file(self, file):
        f = "/" / file.relative_to(PATH_texlive)

        for pattern in self.ignore_patterns:
            if re.fullmatch(pattern, str(f)):
                return False

        return True


    def add_file(self, full_path: Path):

        # Compute digest of original file
        s = hashlib.sha256()
        with open(full_path, "rb") as f:
            content = f.read()
        s.update(content)
        digest = s.digest()

        # The location of this file in the bundle depends on its source.
        target_path = PATH_content
        if full_path.is_relative_to(PATH_texlive):
            target_path /= "texlive" / full_path.relative_to(PATH_texlive)
        elif full_path.is_relative_to(PATH_extra):
            target_path /= "include" / full_path.relative_to(PATH_extra)
        else:
            target_path /= "unknown" / Path(full_path.name)

        self.index.setdefault(full_path.name, []).append(target_path.relative_to(PATH_content))
        target_path.parent.mkdir(parents = True, exist_ok=True)
        shutil.copyfile(full_path, target_path)

        # Apply patches and compute new hash
        if self.has_patch(target_path):
            self.apply_patch(target_path)
            s = hashlib.sha256()
            with open(target_path, "rb") as f:
                s.update(f.read())
            digest = s.digest()

        self.item_shas[target_path.relative_to(PATH_content)] = digest


    def has_patch(self, file):
        return file.name in self.diffs

    # Apply a patch to `file`, if one is provided.
    # We need to copy `file` first, since patching to stdout is tricky.
    def apply_patch(self, file):
        if not self.has_patch(file):
            return False

        self.clearprint(f"Patching {file.name}")
        self.patch_applied_count += 1
        subprocess.run([
            "patch",
            "--quiet",
            "--no-backup",
            file,
            self.diffs[file.name]
        ])

        return True


    # Read include dir, prepare to add files.
    def prepare(self):
        self.extra_basenames = set()
        if PATH_extra.is_dir():
            for f in PATH_extra.rglob("*"):
                if not f.is_file():
                    continue
                if f.suffix == ".diff":
                    n = f.name[:-5] # Cut off ".diff"
                    if n in self.diffs:
                        print(f"Warning: included diff {f.name} has conflicts, ignoring")
                        continue
                    self.diffs[n] = f
                    continue
                if f.name in self.extra_basenames:
                    print(f"Warning: included file {f.name} has conflicts, ignoring")
                    self.extra_conflict_count += 1
                    continue
                self.add_file(f)
                self.extra_count += 1
                self.extra_basenames.add(f.name)

    # Select files
    def add_tree(self, tree_path):
        for f in tree_path.rglob("*"):

            # Update less often so we spend fewer cycles on string manipulation.
            # mod 193 so every digit moves (modding by 100 is boring, we get static zeros)
            if (self.added_count+self.extra_count) % 193 == 0:
                s = f"Selecting files... ({self.added_count+self.extra_count})"
                self.print_len = len(s)
                print(s, end = "\r")

            if not f.is_file():
                continue

            if not self.consider_file(f):
                self.ignored_count += 1
                continue

            # This should be done AFTER consider_file,
            # since we want to increment the counter only if
            # this file wasn't ignored.
            if f.name in self.extra_basenames:
                self.replaced_count += 1
                continue

            self.added_count += 1
            self.add_file(f)

        self.clearprint("Selecting files... Done!")


    # Write auxillary files.
    def finish(self):
        print( "============== Summary ==============")
        print(f"    extra file conflicts: {self.extra_conflict_count}")
        print(f"    files ignored:        {self.ignored_count}")
        print(f"    files replaced:       {self.replaced_count}")
        print(f"    diffs applied/found:  {self.patch_applied_count}/{len(self.diffs)}")
        print( "    =================================")
        print(f"    extra files added:    {self.extra_count}")
        print(f"    total files:          {self.added_count+self.extra_count}")
        print("")

        if len(self.diffs) > self.patch_applied_count:
            print("Warning: not all diffs were applied")

        if len(self.diffs) < self.patch_applied_count:
            print("Warning: some diffs were applied multiple times!")

        print("Preparing auxillary files...", end = "")


        item_shas = list(self.item_shas.items())

        # Sort to guarantee a reproducible hash.
        # Note that the files created below are not hashed!
        item_shas.sort(
            key = lambda x: x[0].name + x[1].hex()
        )

        # Compute and save hash
        self.index["SHA256SUM"] = ["SHA256SUM"]
        with (PATH_content / "SHA256SUM").open("w") as f:
            s = hashlib.sha256()
            for p, d in item_shas:
                s.update(p.name.encode("utf8"))
                s.update(b"\0")
                s.update(d)
                s.update(b"\0")
            f.write(s.hexdigest())

        # This is essentially a detailed version of SHA256SUM,
        # Good for finding file differences between bundles
        with (PATH_output / "file-hashes").open("w") as f:
            for p, d in item_shas:
                f.write(str(p))
                f.write("\t")
                f.write(d.hex())
                f.write("\n")


        # Save search order
        self.index["SEARCH"] = ["SEARCH"]
        shutil.copyfile(
            PATH_bundle / "search-order",
            PATH_content / "SEARCH"
        )

        # Check all conflicts, save those we can't resolve.
        with (PATH_output / "search-report").open("w") as l:
            with (PATH_content / "SEARCH").open("r") as f:
                search = [x.strip() for x in f.readlines()]
                for name, paths in sorted(self.index.items(), key = lambda x: x[0]):
                    if not search_for_file(search, paths):
                        l.write("Will not find:\n")
                        for p in paths:
                            l.write(f"\t{p}\n")
                        l.write("\n\n")

        #if (PATH_install / "TEXLIVE-SHA256SUM").is_file():
        #     shutil.copyfile(
        #        PATH_install / "TEXLIVE-SHA256SUM",
        #        PATH_content / "TEXLIVE-SHA256SUM"
        #    )


        # Save index.
        # Naturally, this must be the last file added to the bundle.
        self.index["INDEX"] = ["INDEX"]
        with (PATH_content / "INDEX").open("w") as f:
            for name, paths in sorted(self.index.items(), key = lambda x: x[0]):
                for p in sorted(paths):
                    digest = self.item_shas.get(p)
                    if digest is None:
                        f.write(f"{name} {p} nohash\n")
                    else:
                        f.write(f"{name} {p} {digest.hex()}\n")

        print(" Done.")



if __name__ == "__main__":
    b = FilePicker()
    b.prepare()
    b.add_tree(PATH_texlive)
    b.finish()
