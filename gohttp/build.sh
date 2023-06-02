#!/bin/sh

set -e 
NAME=libgohttp
LIB_NAME=""

echo "Start building the  platform static library"


cd ffi
if [ "$GOOS" == "windows" ]; then
    LIB_NAME=$NAME.lib
else 
    LIB_NAME=$NAME.a
fi
GOOS=$GOOS GOARCH=$GOARCH CGO_ENABLED=1 go build -ldflags "-s -w" -buildmode=c-archive -o $LIB_NAME
cd -
