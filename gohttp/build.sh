#!/bin/sh

set -e 

echo "Start building the  platform static library"

cd ffi
go build -ldflags "-s -w" -buildmode=c-archive -o libgohttp.a
cd -
