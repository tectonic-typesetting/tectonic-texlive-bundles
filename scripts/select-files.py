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



# Input paths
PATH_ignore  = PATH_bundle / "ignore"
PATH_extra   = PATH_bundle / "include"
PATH_install = Path(f"build/install/{VAR_bundlename}")
PATH_texlive = PATH_install / "texmf-dist"

# Output paths
PATH_output  = Path(f"build/output/{VAR_bundlename}")
PATH_content = PATH_output / "content"





class FilePicker(object):
    def __init__(self):
        self.item_shas = {}
        self.final_hexdigest = None

        # Map of "filename": Path
        self.index = {}

        # Keeps track of conflicting file names
        # { "basename": {
        #       b"digest": Path(fullpath)
        # }}
        self.clashes = {}

        # Number of files with conflicting file names
        # that have the same content as the file we added to the bundle.
        # These are not added to self.clashes.
        self.identical_clashes = 0


        # Hashes of files we've applied patches to
        # (calculated before patching)
        # This lets us detect conflicting patched files
        # with identical contents.
        self.pre_patch_shas = set()

        # Array of diff file paths in include dir.
        # Scanned at start of run, applied while running.
        # Map of "filename": Path(filename.diff)
        self.diffs = {}
        self.patch_applied_count = 0 # How many diffs we've applied

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


    def consider_file(self, file):
        """
        Consider adding the specified TeXLive file to the installation tree.
        This is where all the nasty hairy logic will accumulate that enables us
        to come out with a nice pretty tarball in the end.
        """

        f = "/" / file.relative_to(PATH_texlive)

        for pattern in self.ignore_patterns:
            if re.fullmatch(pattern, str(f)):
                return False

        return True


    def add_file(self, full_path: Path):
        # Have we seen this filename before?
        prev_tuple = self.item_shas.get(full_path.name)

        # Compute digest of original file
        s = hashlib.sha256()
        with open(full_path, "rb") as f:
            content = f.read()
        s.update(content)
        digest = s.digest()


        if prev_tuple is None:
            # This is a new file, ok for now.

            # Edge case: this is a duplicate of a file we've already patched.
            # This hash won't show up in self.item_shas, since that dict
            # only contains the post-patch hash.
            if digest in self.pre_patch_shas:
                self.identical_clashes += 1
                return

            target_path = PATH_content
            if self.has_patch(full_path):
                target_path /= "patched" / Path(full_path.name)
            elif full_path.is_relative_to(PATH_texlive):
                target_path /= "texlive" / full_path.relative_to(PATH_texlive)
            elif full_path.is_relative_to(PATH_extra):
                target_path /= "include" / full_path.relative_to(PATH_extra)
            else:
                target_path /= "unknown" / Path(full_path.name)

            self.index[full_path.name] = target_path.relative_to(PATH_content)
            target_path.parent.mkdir(parents = True, exist_ok=True)
            shutil.copyfile(full_path, target_path)

            # Apply patches and compute new hash
            if self.has_patch(target_path):
                self.apply_patch(target_path)
                self.pre_patch_shas.add(digest)
                s = hashlib.sha256()
                with open(target_path, "rb") as f:
                    s.update(f.read())
                digest = s.digest()

            self.item_shas[full_path.name] = (digest, full_path)

        # Don't check contents, conflicting identical files are still a conflict.
        # we'll have to explicitly ignore one version.
        elif prev_tuple[0] != digest:
            # We already have a file with this name and different contents
            bydigest = self.clashes.setdefault(full_path.name, {})

            if not len(bydigest):
                # If this is the first duplicate, don't forget that we've seen
                # the file at least once before.
                bydigest[prev_tuple[0]] = [prev_tuple[1]]

            pathlist = bydigest.setdefault(digest, [])
            pathlist.append(full_path)

        else:
            self.identical_clashes += 1


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

    def go(self):
        # Statistics for summary
        extra_count = 0 # Extra files added
        extra_conflict_count = 0 # Number of conflicting extra files (0, ideally)
        texlive_count = 0 # Number of files from texlive
        ignored_count = 0 # Number of texlive files ignored
        replaced_count = 0 # Number of texlive files replaced with extra files

        # Add extra files
        extra_basenames = set()
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
                if f.name in extra_basenames:
                    print(f"Warning: included file {f.name} has conflicts, ignoring")
                    extra_conflict_count += 1
                    continue
                self.add_file(f)
                extra_count += 1
                extra_basenames.add(f.name)

        # Add the main tree.
        for f in PATH_texlive.rglob("*"):

            # Update less often so we spend fewer cycles on string manipulation.
            # mod 193 so every digit moves (modding by 100 is boring, we get static zeros)
            if (texlive_count+extra_count) % 193 == 0:
                s = f"Selecting files... ({texlive_count+extra_count})"
                self.print_len = len(s)
                print(s, end = "\r")

            if not f.is_file():
                continue

            if not self.consider_file(f):
                ignored_count += 1
                continue

            # This should be done AFTER consider_file,
            # since we want to increment the counter only if
            # this file wasn't ignored.
            if f.name in extra_basenames:
                replaced_count += 1
                continue

            texlive_count += 1
            self.add_file(f)
                

        print("Selecting files... Done! Summary is below.")
        print(f"\textra file conflicts: {extra_conflict_count}")
        print(f"\ttl files ignored:     {ignored_count}")
        print(f"\ttl files replaced:    {replaced_count}")
        print(f"\ttl filename clashes:  {len(self.clashes)}")
        print(f"\ttl identical clashes: {self.identical_clashes}")
        print(f"\tdiffs applied/found:  {self.patch_applied_count}/{len(self.diffs)}")
        print( "\t===============================")
        print(f"\textra files added:    {extra_count}")
        print(f"\ttotal files:          {texlive_count+extra_count}")
        print("")

        if len(self.diffs) != self.patch_applied_count:
            print("Warning: not all diffs were applied")

        # Compute content hash
        s = hashlib.sha256()
        s.update(struct.pack(">I", len(self.item_shas)))
        s.update(b"\0")
        for name in sorted(self.item_shas.keys()):
            s.update(name.encode("utf8"))
            s.update(b"\0")
            s.update(self.item_shas[name][0])
        self.final_hexdigest = s.hexdigest()

        # Write bundle metadata
        with (PATH_content / "SHA256SUM").open("w") as f:
            f.write(self.final_hexdigest)
        if (PATH_install / "TEXLIVE-SHA256SUM").is_file():
            shutil.copyfile(
                PATH_install / "TEXLIVE-SHA256SUM",
                PATH_content / "TEXLIVE-SHA256SUM"
            )
        with (PATH_content / "INDEX").open("w") as f:
            for k, p in sorted(self.index.items(), key = lambda x: x[0]):
                f.write(f"{k} {p}\n")



        # Write debug files

        # This is essentially a detailed version of SHA256SUM,
        # Good for detecting file differences between bundles
        with (PATH_output / "file-hashes").open("w") as f:
            f.write(f"{len(self.item_shas)}\n")
            for name in sorted(self.item_shas.keys()):
                f.write(name)
                f.write("\t")
                f.write(self.item_shas[name][0].hex())
                f.write("\n")


        with (PATH_output / "listing").open("w") as f:
            for base in sorted(self.item_shas.keys()):
                f.write(base+"\n")

        if len(self.clashes):
            print(f"Warning: {len(self.clashes)} file clashes were found.")
            print(f"Logging clash report to {PATH_output}/clash-report")

            with (PATH_output / "clash-report").open("w") as f:
                for filename in sorted(self.clashes.keys()):
                    f.write(f"{filename}:\n")
                    bydigest = self.clashes[filename]

                    for digest in sorted(bydigest.keys()):
                        f.write(f"\t{digest.hex()[:8]}:\n")

                        for path in sorted(bydigest[digest]):
                            f.write(f"\t\t{path}\n")
                    f.write("\n\n")




if __name__ == "__main__":
    b = FilePicker()
    b.go()
