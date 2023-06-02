#!/bin/bash

declare -A target_map

target_map[x86_64-unknown-linux-musl]="x86_64-linux-gnu-gcc,GOOS=linux,GOARCH=amd64"
target_map[aarch64-unknown-linux-musl]="aarch64-linux-gnu-gcc,GOOS=linux,GOARCH=arm64"
target_map[x86_64-apple-darwin]="GOOS=darwin,GOARCH=amd64"
target_map[aarch64-apple-darwin]="GOOS=darwin,GOARCH=arm64"
target_map[aarch64-apple-darwin]="GOOS=darwin,GOARCH=arm64"

echo "${target_map["x86_64-unknown-linux-musl"]}"