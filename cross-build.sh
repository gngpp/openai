#!/bin/bash

cargo update

target_list=(x86_64-unknown-linux-musl aarch64-unknown-linux-musl)
for target in ${target_list[@]}; do
    cargo zigbuild --release --target $target
done