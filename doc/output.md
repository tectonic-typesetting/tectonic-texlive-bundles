# Output Files

All the files that the bundle-builder script produces are listed below.



**`./build.sh <bundle> select` produces the following:**
 - `./build/output/<bundle>/content`: contains all bundle files. It is organized by source: files from the bundle's `include` dir will be under `./include`, texlive files will be under `./texlive`, and so on. See `builder/src/select.rs`.
 This directory also contains some metadata:
   - `content/FILES`: each line of this file is `<path> <hash>`, sorted by file name.\
   Files with identical names are included.\
   Files not in any search path are also included.\
   `<hash>` is either a hex sha256 of that file's contents, or `nohash` for a few special files.
   - `content/SHA256SUM`: The sha256sum of `content/FILES`. This string uniquely defines this bundle.
   - `content/SEARCH`: File search order for this bundle. See bundle spec documentation.
 - `search-report`: debug file. Lists all directories that will not be searched by the rules in `search-order`.\
  The entries in this file are non-recursive: If `search-report` contains a line with `/texlive`, this means that direct children of `/texlive` (like `/texlive/file.tex`) will not be found, but files in *subdirectories* (like `/texlive/tex/file.tex`) may be.


**`./build.sh <bundle> ttbv1` produces the following:**
 - `<bundle>.ttb`: the bundle. Note that the ttb version is *not* included in the extension.
   - Index location and length are printed once this job completes.
   - You can extract files from this bundle by running `dd if=file.ttb ibs=1 skip=<start> count=<len> | gunzip`



