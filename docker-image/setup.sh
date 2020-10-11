#! /bin/bash
# Copyright 2016-2020 the Tectonic Project.
# Licensed under the MIT License.
#
# Set up an image that's ready to generate TeXLive packages reproducibly.

deps="
libdigest-perl-md5-perl
libfontconfig1
git-core
python3
python3-toml
wget
"

set -ex
apt-get update
DEBIAN_FRONTEND=noninteractive apt-get install -yq --no-install-recommends $deps
rm -rf /var/lib/apt/lists/*
rm -f /setup.sh  # self-destruct!
