#!/bin/bash

set -ex

cargo build --release -j66
docker buildx build -t deeplythink/ninja:v0.0.1 . --push
