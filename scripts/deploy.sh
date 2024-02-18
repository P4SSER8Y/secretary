#!/usr/bin/env bash

if [ ! -z "$DEBUG" ]; then
    DRY_RUN="--dry-run"
    set -e
    set -x
fi

pushd $(dirname $0)/../dist
base=$(pwd)
echo $base

set -x
rsync $DRY_RUN -avP --delete $base/ui/ $1/ui/
rsync $DRY_RUN -avP --exclude="/**/" $base/ $1/
