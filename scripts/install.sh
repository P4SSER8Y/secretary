#!/usr/bin/env bash

set -e

pushd $(dirname $0)/..
base=$(pwd)

cargo update

cd $base/ui
yarn install

popd
