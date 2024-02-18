#!/usr/bin/env bash

set -e
source $(dirname $0)/utils.sh

pushd $(dirname $0)/..
base=$(pwd)

target=$(get_target $1)
if [ -z "$target" ]; then
    echo "please select build target"
    exit 2
fi
echo build target: $target

cd $base
cargo build --target $target --release --target-dir $base/target

mkdir -p $base/dist
cp $base/Rocket.toml $base/dist/Rocket.toml
cp $base/target/$target/release/secretary $base/dist/secretary

cd $base/ui
yarn build

popd
