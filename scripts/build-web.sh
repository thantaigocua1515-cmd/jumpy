#!/usr/bin/env bash

#
# This script is usually run by the justfile
#

is_release="$1"
target=wasm32-unknown-unknown
target_dir="web-target"

release_arg=""
build_kind="debug"
dist_dir="$target_dir/wasm-debug"

if [ "$is_release" == "release" ]; then
    release_arg="--release"
    build_kind="release"
    dist_dir="$target_dir/wasm-release"
fi

export CARGO_TARGET_DIR=$target_dir

set -ex

cargo build --target $target $release_arg
rm -rf $dist_dir
mkdir -p $dist_dir
wasm-bindgen --out-dir $dist_dir --target web --no-typescript $target_dir/$target/$build_kind/jumpy.wasm
cp wasm_resources/index.html $dist_dir/index.html
# Mobile shell extras (service worker cache + PWA manifest), if present
cp wasm_resources/sw.js $dist_dir/sw.js 2>/dev/null || true
cp wasm_resources/manifest.webmanifest $dist_dir/manifest.webmanifest 2>/dev/null || true
# Shrink the WASM binary when wasm-opt (binaryen) is available
if command -v wasm-opt >/dev/null 2>&1; then
    wasm-opt -Oz --all-features --strip-debug --strip-producers -o $dist_dir/jumpy_bg.wasm.tmp $dist_dir/jumpy_bg.wasm
    mv $dist_dir/jumpy_bg.wasm.tmp $dist_dir/jumpy_bg.wasm
fi
cp -r assets $dist_dir
