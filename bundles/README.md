# Bundle Specification

Back to [`README.md`](../README.md)



## Contents
A bundle directory contains the following:
 - `bundle.sh`: bundle metadata, stored in bash variables
 - `ignore`: ignore patterns. TeXlive installation files matching any of these will not be included in the bundle.
 - `tl-profile.txt`: the TeXlive profile to install. See TeXlive docs. \
 Note that all paths are replaced with `@dest@`, which is replaced with a path by the docker build script.
 - `include/`: extra files to include in the bundle. All files are read, including those in subdirectories. \
 If a filename in `include/` conflicts with a file in TeXlive, the `include/` file is used and a warning is printed.




## Metadata: `bundle.sh`
An example file with comments is below. All the variables below must be present. This is enforced by `./build.sh`, make sure you update the checks there if you add any variables.
```sh
# This bundle's name. Should probably match subdirectory.
bundle_name="texlive2023.0r0"

# Fake time for texlive installation.
# Changing this will change the output hash.
# Format must be "YYYY-MM-DD HH:MM:SS".
# Make sure this is dated AFTER the texlive release,
# otherwise tar will complain about future timestamps.
bundle_faketime="2023-10-10 00:00:00"

# Compute this hash by running `sha256 -b file.iso`
# If this is an empty string, hash is not checked.
# Do not include the file name in this string.
#
# It's also a good idea to add a comment with the file name
# and TeXlive version number of this image, so that others
# may find it.
bundle_texlive_hash="620923de5936ab315926e81de2cb8253a9c626fb7e03d8ffe0d424598eb32f94"

# The SHA256SUM we should get once this bundle is built.
# Will change if bundle_faketime is changed, or if container
# updates change the UUIDs in TeX files.
bundle_result_hash="209d4b6a220bec2d1e2e89c7ba0dbe02b0e6f2416abce5fb8df228e06cf1e335"
```




## Ignoring files: `ignore`
Any path that matches a line in this file is ignored.
Leading and trailing whitespace is ignored, empty lines are ignored.
Comments start with a `#` and *must* be on their own line.

The format here is **NOT** similar to the format of a `.gitignore`!\
Each line is a proper [regex pattern](https://regexr.com/). Watch out for the following:
 - `*`'s need a token to repeat (probably `.*`)
 - literal dots must be escaped (like `\.`)


Matching is implemented using python's re.match(), which evaluates to true if the *whole path* matches
this regular expression. All paths are relative to texmf-dist. For example, when deciding whether to include
`/path/to/texmf-dist/tex/file.tex`, we will try to match `/tex/file.tex`.

### A few example patterns:
 - `/tex/.*`: Ignore everything under `texmf-dist/tex`
 - `.*\.log`: Ignore all paths ending in `.log`
 - `fonts`: Nothing will match this pattern. All paths begin with at least a `/`
 - `/fonts`: Only the file `/fonts` will match this pattern. Subfiles of a directory called `fonts` will *not* match, because the whole string must match. The correct way to ignore the `fonts` directory is with the pattern `/fonts/.*`.