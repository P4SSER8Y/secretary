#!/usr/bin/env sh

set -e
source $(dirname $0)/utils.sh

pushd $(dirname $0)/..

target=$(get_target $1)
rustup target add $target

base=$(pwd)
cargo update

cd $base/ui
yarn install

popd
