# Bundle Specification

Back to [`README.md`](../README.md)



## Contents
A bundle directory contains the following:
 - `bundle.sh`: bundle metadata, stored in bash variables
 - `ignore`: ignore patterns. TeXlive installation files matching any of these will not be included in the bundle.
 - `search-paths`: directory "search order." These rules influences how tectonic resolves filename conflicts.
 - `tl-profile.txt`: the TeXlive profile to install. See TeXlive docs. \
 Note that all paths are replaced with `@dest@`, which is replaced with a path by the docker build script.
 - `include/`: extra files to include in the bundle. All files are read, including those in subdirectories. \
 This directory may also contain diffs, see below. Files ending in `.diff` are special.




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
 - literal dots should be escaped (like `\.`)


Matching is implemented using python's re.match(), which evaluates to true if the *whole path* matches
this regular expression. All paths are relative to texmf-dist. For example, when deciding whether to include
`/path/to/texmf-dist/tex/file.tex`, we will try to match `/tex/file.tex` against a line in the ignore file.

### A few example patterns:
 - `/tex/.*`: Ignore everything under `texmf-dist/tex`
 - `.*\.log`: Ignore all paths ending in `.log`
 - `fonts`: Nothing will match this pattern. All paths begin with at least a `/`
 - `/fonts`: Only the file `/fonts` will match this pattern. Subfiles of a directory called `fonts` will *not* match, because the whole string must match. The correct way to ignore the `fonts` directory is with the pattern `/fonts/.*`.



## Adding files: `include/`

Any files in this directory will be added to the bundle. Subdirectories are traversed and ignored (we pretend the directory structure is flat). If a filename here conflicts with a file in TeXlive, the TeXlive version is **silently** ignored.

Any file that ends with `.diff` is special. If the file selector encounters `a.diff`, it will NOT copy `a.diff` into the bundle. Instead, it will apply `a.diff` when it encounters a file named `a`.

To make a diff file, run `diff <texlive-file> <modified-file>`. ORDER MATTERS! \
Diffs are applied via a simple call to `patch <file> <diff>`. See [`select-files.py`](../scripts/select-files.py).



## Finding files: `search-paths`

### Overview

Any TeX distribution needs a way to find files. This is necessary because files are usually included only by name: `\include{file}`, `\usepackage{package}`, etc. Where do we find `file.tex` and `package.sty`?

In a conventional TeXLive installation, kpathsea solves this problem. It defines an array of "search paths," and walks through them when you ask for a file. You can find an overview [here](https://www.overleaf.com/learn/latex/Articles/An_introduction_to_Kpathsea_and_how_TeX_engines_search_for_files) and more detailed information in the kpathsea docs.

Tectonic's supporting files are distributed in bundles, so we can't use the same approach.
Within tectonic's *bundles*[^1], we use INDEX and SEARCH files to map a filename to an input path. Note that this logic is implemented in tectonic, not in the bundle build script.

[^1]: Tectonic searches for files on your disk seperately. The information in this file only applies to bundles. I won't document this fully here, you'll have to read the tectonic docs and source code.

- **Case 1:** tectonic looks for `file.tex` and finds one path in `INDEX`\
  Nothing fancy here, we just use the file we found.

- **Case 2:** tectonic looks for `partial/path/to/file.tex`\
  This is an edge case caused by some packages (for example, `fithesis`). To handle this,
  we first find `file.tex` in `INDEX` and look at its path. If its path ends with `partial/path/to/file.tex`, we use it,
  if it doesn't, we don't. If multiple files match, we print an error--that shouldn't ever happen.

- **Case 3:** tectonic looks for `file.tex` and finds multiple paths in `INDEX`\
This where things get interesting. First, we match all paths against each line of the bundles's `SEARCH` file with a simple `starts_with`.
  - If *exactly one* path matches a certain line, we immediately stop checking and use that path. Search lines are ordered by priority, so if only one path matches the first line, it *must* be the right path to use.
  - If multiple paths match a certain line, we discard all others and resolve the conflict alphabetically.
  - If we've checked all lines of `SEARCH` and found no matches, we also resolve alphabetically.

"Resolving the conflict alphabetically" means we sort the paths in alphabetical order and pick the first. This emulates an alphabetically-ordered DFS on the file tree, which is a reasonable default.

Any filename conflicts which would be resolved alphabetically are listed in `search-report` after the `content` build step. These aren't errors, but we should look over that file to make sure everything is working as expected.


### Defining search paths

Search paths are defined in `<bundle>/search-paths`. This file is directly copied into `SEARCH` in the bundle.\
It is a simple list of paths, relative to the bundle root directory, ordered by decreasing priority.

Just like kpathsea search paths, each line can end with one or two slashes.

 - If a line ends with two slashes (like `texlive/tex/latex//`), it will match all subdirectories of that path.
 - If a line ends with one slash (like `texlive/tex/latex/`), it will match only direct children of that path:\
 `texlive/tex/latex/a.tex` will match, `texlive/tex/latex/base/a.tex` will not.
 - If a line does not end with a slash, we pretend it ends with one.
 - If a line ends with three or more slashes, it won't match any paths. Don't do that.

This scheme lets us override the default "alphabetic DFS" by adding seach paths as follows, which will look for direct children of `latex` before descending into subdirectories.
```
texlive/tex/latex/
texlive/tex/latex//
```

Be careful--this file is NOT checked for correctness (yet). It must have no comments and no empty lines.