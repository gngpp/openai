#!/bin/bash

set -ex

if [ -n "$GIT_TOKEN" ]; then
    if [ -d "patches" ]; then
        rm -rf patches
    fi
    git clone https://gngpp:$GIT_TOKEN@github.com/gngpp/ninja-patches patches

    if [ $(ls patches/*.patch 2>/dev/null | wc -l) -gt 0 ]; then
        for patch in patches/*.patch; do
            git apply --whitespace=nowarn "$patch"
        done
    fi
fi

root=$(pwd)
: ${tag=latest}
: ${rmi=false}
: ${os=linux}
[ ! -d uploads ] && mkdir uploads

# Separate arrays for target architectures and Docker images
target_architectures=("x86_64-unknown-linux-musl" "aarch64-unknown-linux-musl" "armv7-unknown-linux-musleabi" "armv7-unknown-linux-musleabihf" "arm-unknown-linux-musleabi" "arm-unknown-linux-musleabihf" "armv5te-unknown-linux-musleabi" "i686-unknown-linux-gnu" "i586-unknown-linux-gnu" "x86_64-pc-windows-msvc")
docker_images=("getsentry/rust-musl-cross:x86_64-musl" "ghcr.io/gngpp/rust-musl-cross:aarch64-musl" "ghcr.io/gngpp/rust-musl-cross:armv7-musleabi" "ghcr.io/gngpp/rust-musl-cross:armv7-musleabihf" "ghcr.io/gngpp/rust-musl-cross:arm-musleabi" "ghcr.io/gngpp/rust-musl-cross:arm-musleabihf" "ghcr.io/gngpp/rust-musl-cross:armv5te-musleabi" "ghcr.io/gngpp/rust-musl-cross:i686-musl" "ghcr.io/gngpp/rust-musl-cross:i586-musl" "ghcr.io/gngpp/cargo-xwin:latest")

get_docker_image() {
    local target_arch="$1"
    local index
    for ((index = 0; index < ${#target_architectures[@]}; ++index)); do
        if [ "${target_architectures[index]}" == "$target_arch" ]; then
            echo "${docker_images[index]}"
            return 0
        fi
    done

    echo "Architecture not found"
    return 1
}

rmi_docker_image() {
    echo "Removing $1"
    docker rmi $1
}

build_macos_target() {
    cargo build --release --target $1 --features mimalloc
    sudo chmod -R 777 target
    cd target/$1/release
    tar czvf ninja-$tag-$1.tar.gz ninja
    shasum -a 256 ninja-$tag-$1.tar.gz >ninja-$tag-$1.tar.gz.sha256
    mv ninja-$tag-$1.tar.gz $root/uploads/
    mv ninja-$tag-$1.tar.gz.sha256 $root/uploads/
    cd -
}

build_linux_target() {
    features=""
    if [ "$1" = "armv5te-unknown-linux-musleabi" ] || [ "$1" = "arm-unknown-linux-musleabi" ] || [ "$1" = "arm-unknown-linux-musleabihf" ]; then
        features="--features rpmalloc"
    else
        if [ "$1" = "i686-unknown-linux-gnu" ] || [ "$1" = "i586-unknown-linux-gnu" ]; then
            features=""
        else
            features="--features mimalloc"
        fi
    fi

    docker_image=$(get_docker_image "$1")

    docker run --rm -t --user=$UID:$(id -g $USER) \
        -v $(pwd):/home/rust/src \
        -v $HOME/.cargo/registry:/root/.cargo/registry \
        -v $HOME/.cargo/git:/root/.cargo/git \
        -e "FEATURES=$features" \
        -e "TARGET=$1" \
        $docker_image /bin/bash -c "cargo build --release --target \$TARGET  \$FEATURES"

    sudo chmod -R 777 target
    if [ "$1" != "i686-unknown-linux-gnu" ] && [ "$1" != "i586-unknown-linux-gnu" ]; then
        upx --best --lzma target/$1/release/ninja
    fi
    cd target/$1/release
    tar czvf ninja-$tag-$1.tar.gz ninja
    shasum -a 256 ninja-$tag-$1.tar.gz >ninja-$tag-$1.tar.gz.sha256
    mv ninja-$tag-$1.tar.gz $root/uploads/
    mv ninja-$tag-$1.tar.gz.sha256 $root/uploads/
    cd -
}

build_windows_target() {
    docker_image=$(get_docker_image "$1")

    docker run --rm -t \
        -v $(pwd):/home/rust/src \
        -v $HOME/.cargo/registry:/usr/local/cargo/registry \
        -v $HOME/.cargo/git:/usr/local/cargo/git \
        $docker_image cargo xwin build --release --target $1

    sudo chmod -R 777 target
    upx --best --lzma target/$1/release/ninja.exe
    cd target/$1/release
    tar czvf ninja-$tag-$1.tar.gz ninja.exe
    shasum -a 256 ninja-$tag-$1.tar.gz >ninja-$tag-$1.tar.gz.sha256
    mv ninja-$tag-$1.tar.gz $root/uploads/
    mv ninja-$tag-$1.tar.gz.sha256 $root/uploads/
    cd -
}

if [ "$os" = "windows" ]; then
    target_list=(x86_64-pc-windows-msvc)
    for target in "${target_list[@]}"; do
        echo "Building $target"

        docker_image=$(get_docker_image "$target")

        build_windows_target "$target"

        if [ "$rmi" = "true" ]; then
            rmi_docker_image "$docker_image"
        fi
    done
fi

if [ "$os" = "linux" ]; then
    target_list=(x86_64-unknown-linux-musl aarch64-unknown-linux-musl armv7-unknown-linux-musleabi armv7-unknown-linux-musleabihf armv5te-unknown-linux-musleabi arm-unknown-linux-musleabi arm-unknown-linux-musleabihf i686-unknown-linux-gnu i586-unknown-linux-gnu)

    for target in "${target_list[@]}"; do
        echo "Building $target"

        docker_image=$(get_docker_image "$target")

        if [ "$target" = "x86_64-pc-windows-msvc" ]; then
            build_windows_target "$target"
        else
            build_linux_target "$target"
        fi

        if [ "$rmi" = "true" ]; then
            rmi_docker_image "$docker_image"
        fi
    done
fi

if [ "$os" = "macos" ]; then
    if ! which upx &>/dev/null; then
        brew install upx
    fi
    rustup target add x86_64-apple-darwin aarch64-apple-darwin
    target_list=(x86_64-apple-darwin aarch64-apple-darwin)
    for target in "${target_list[@]}"; do
        echo "Building $target"
        build_macos_target "$target"
    done
fi

generate_directory_tree() {
    find "$1" -print | sed -e 's;[^/]*/;|____;g;s;____|; |;g'
}

generate_directory_tree "uploads"
