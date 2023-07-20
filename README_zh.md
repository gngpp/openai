<br>简体中文 | [English](README.md)

[![CI](https://github.com/gngpp/opengpt/actions/workflows/CI.yml/badge.svg)](https://github.com/gngpp/opengpt/actions/workflows/CI.yml)
[![CI](https://github.com/gngpp/opengpt/actions/workflows/Release.yml/badge.svg)](https://github.com/gngpp/opengpt/actions/workflows/Release.yml)
	<a target="_blank" href="https://github.com/gngpp/vdns/blob/main/LICENSE">
		<img src="https://img.shields.io/badge/license-MIT-blue.svg"/>
	</a>
  <a href="https://github.com/gngpp/opengpt/releases">
    <img src="https://img.shields.io/github/release/gngpp/opengpt.svg?style=flat">
  </a><a href="https://github.com/gngpp/opengpt/releases">
    <img src="https://img.shields.io/github/downloads/gngpp/opengpt/total?style=flat">
  </a>
  [![](https://img.shields.io/docker/image-size/gngpp/opengpt)](https://registry.hub.docker.com/r/gngpp/opengpt)
  [![Docker Image](https://img.shields.io/docker/pulls/gngpp/opengpt.svg)](https://hub.docker.com/r/gngpp/opengpt/)

# opengpt

一个逆向工程的非官方的 `ChatGPT` 代理（绕过 Cloudflare 403 Access Denied）

- API密钥获取
- 电子邮件/密码帐户认证 (由于作者没有账号，暂不支持Google/微软第三方登录)
- Unofficial/Official Http API 代理 (供第三方客户端接入)
- 原汁原味的ChatGPT WebUI

> 局限性: 无法绕过 OpenAI 的彻底 IP 禁令

### 平台支持

- Linux musl 目前支持
  - `x86_64-unknown-linux-musl`
  - `aarch64-unknown-linux-musl`
  - `armv7-unknown-linux-musleabi`
  - `armv7-unknown-linux-musleabihf`
  - `arm-unknown-linux-musleabi`
  - `arm-unknown-linux-musleabihf`
  - `armv5te-unknown-linux-musleabi`
- Windows 目前支持
  - `x86_64-pc-windows-msvc`
- MacOS 目前支持
  - `x86_64-apple-darwin`
  - `aarch64-apple-darwin`

### 安装
  >
  > #### Ubuntu(Other Linux)

  GitHub [Releases](https://github.com/gngpp/opengpt/releases/latest) 中有预编译的 deb包，二进制文件，以Ubuntu为例：

```shell
wget https://github.com/gngpp/opengpt/releases/download/v0.2.0/opengpt-0.2.0-x86_64-unknown-linux-musl.deb

dpkg -i opengpt-0.2.0-x86_64-unknown-linux-musl.deb

opengpt serve run
```

  > #### Docker

```shell
docker run --rm -it -p 7999:7999 --hostname=opengpt \
  -e OPENGPT_WORKERS=1 \
  -e OPENGPT_LOG_LEVEL=info \
  gngpp/opengpt:latest serve run
```

 > #### OpenWrt

GitHub [Releases](https://github.com/gngpp/opengpt/releases/latest) 中有预编译的 ipk 文件， 目前提供了 aarch64/x86_64 等架构的版本，下载后使用 opkg 安装，以 nanopi r4s 为例：

```shell
wget https://github.com/gngpp/opengpt/releases/download/v0.2.0/opengpt_0.2.0_aarch64_generic.ipk
wget https://github.com/gngpp/opengpt/releases/download/v0.2.0/luci-app-opengpt_1.0.2-1_all.ipk
wget https://github.com/gngpp/opengpt/releases/download/v0.2.0/luci-i18n-opengpt-zh-cn_1.0.2-1_all.ipk

opkg install opengpt_0.2.0_aarch64_generic.ipk
opkg install luci-app-opengpt_1.0.2-1_all.ipk
opkg install luci-i18n-opengpt-zh-cn_1.0.2-1_all.ipk
```

### Command Line(dev)

### Http 服务

- 原汁原味ChatGPT WebUI
- 支持非官方/官方API代理
- API前缀与官方一致
- 可接入第三方客户端
- 可接入IP代理池，提高并发量
- API文档
  - Platfrom API [doc](https://platform.openai.com/docs/api-reference)
  - Backend API [doc](doc/rest.http)

- 参数说明
  - `--level`，环境变量 `OPENGPT_LOG_LEVEL`，日志级别: 默认info
  - `--host`，环境变量 `OPENGPT_HOST`， 服务监听地址: 默认0.0.0.0，
  - `--port`，环境变量 `OPENGPT_PORT`， 监听端口: 默认7999
  - `--workers`，环境变量 `OPENGPT_WORKERS`， 工作线程: 默认1
  - `--tls-cert`，环境变量 `OPENGPT_TLS_CERT`，TLS证书公钥，支持格式: EC/PKCS8/RSA
  - `--tls-key`，环境变量 `OPENGPT_TLS_KEY`，TLS证书私钥
  - `--proxies`，环境变量 `OPENGPT_PROXY`，代理，支持代理池，格式: protocol://user:pass@ip:port

```shell
$ opengpt serve --help
Start the http server

Usage: opengpt serve run [OPTIONS]

Options:
  -H, --host <HOST>
          Server Listen host [env: OPENGPT_HOST=] [default: 0.0.0.0]
  -P, --port <PORT>
          Server Listen port [env: OPENGPT_PORT=] [default: 7999]
  -W, --workers <WORKERS>
          Server worker-pool size (Recommended number of CPU cores) [env: OPENGPT_WORKERS=] [default: 1]
  -L, --level <LEVEL>
          Log level (info/debug/warn/trace/error) [env: OPENGPT_LOG_LEVEL=] [default: info]
      --proxies <PROXIES>
          Server proxies pool, example: protocol://user:pass@ip:port [env: OPENGPT_PROXY=]
      --tcp-keepalive <TCP_KEEPALIVE>
          TCP keepalive (second) [env: OPENGPT_TCP_KEEPALIVE=] [default: 5]
      --tls-cert <TLS_CERT>
          TLS certificate file path [env: OPENGPT_TLS_CERT=]
      --tls-key <TLS_KEY>
          TLS private key file path (EC/PKCS8/RSA) [env: OPENGPT_TLS_KEY=]
  -S, --sign-secret-key <SIGN_SECRET_KEY>
          Enable url signature (signature secret key) [env: OPENGPT_SIGNATURE=]
  -T, --tb-enable
          Enable token bucket flow limitation [env: OPENGPT_TB_ENABLE=]
      --tb-store-strategy <TB_STORE_STRATEGY>
          Token bucket store strategy (mem/redis) [env: OPENGPT_TB_STORE_STRATEGY=] [default: mem]
      --tb-capacity <TB_CAPACITY>
          Token bucket capacity [env: OPENGPT_TB_CAPACITY=] [default: 60]
      --tb-fill-rate <TB_FILL_RATE>
          Token bucket fill rate [env: OPENGPT_TB_FILL_RATE=] [default: 1]
      --tb-expired <TB_EXPIRED>
          Token bucket expired (second) [env: OPENGPT_TB_EXPIRED=] [default: 86400]
  -h, --help
          Print help
```

### 自行编译

- Linux编译，Ubuntu机器为例:

```shell
# 本机编译
git clone https://github.com/gngpp/opengpt.git && cd opengpt
./build.sh

# 跨平台编译，依赖于docker(如果您可以自己解决跨平台编译依赖)，编译所有支持平台
./build_cross.sh # 默认使用docker构建linux/windows平台
os=macos ./build_cross.sh # 默认在Macos上构建Macos平台

# 编译单个平台二进制，以 aarch64-unknown-linux-musl 为例:
docker run --rm -it \
  -v $(pwd):/home/rust/src \
  -v $HOME/.cargo/registry:/root/.cargo/registry \  # 如果你希望使用本地缓存
  -v $HOME/.cargo/git:/root/.cargo/git \  # 如果你希望使用本地缓存
  ghcr.io/gngpp/opengpt-builder:aarch64-unknown-linux-musl \
  cargo build --release
```

- OpenWrt 编译

```shell
cd package
svn co https://github.com/gngpp/opengpt/trunk/openwrt
cd -
make menuconfig # choose LUCI->Applications->luci-app-opengpt  
make V=s
```

### 预览

![img0](./doc/img/img0.png)
![img1](./doc/img/img1.png)
![img2](./doc/img/img2.png)

### 参考

- <https://github.com/tjardoo/openai-client>
- <https://github.com/jpopesculian/reqwest-eventsource>
