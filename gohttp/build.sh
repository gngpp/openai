#!/bin/sh

set -e 

echo "Start building the  platform static library"

cd ffi
GOOS=$GOOS GOARCH=$GOARCH CGO_ENABLED=1 go build -ldflags "-s -w" -buildmode=c-archive -o libgohttp.a
cd -
