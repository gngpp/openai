<br>English | [简体中文](https://github.com/gngpp/ninja/blob/main/doc/readme_zh.md)

If the project is helpful to you, please consider [donating support](https://github.com/gngpp/gngpp/blob/main/SPONSOR.md#sponsor-my-open-source-works) for continued project maintenance, or you can Pay for consulting and technical support services.

### Install

- #### Platform

  - `x86_64-unknown-linux-musl`
  - `aarch64-unknown-linux-musl`
  - `armv7-unknown-linux-musleabi`
  - `armv7-unknown-linux-musleabihf`
  - `arm-unknown-linux-musleabi`
  - `arm-unknown-linux-musleabihf`
  - `armv5te-unknown-linux-musleabi`
  - `i686-unknown-linux-gnu`
  - `i586-unknown-linux-gnu`
  - `x86_64-pc-windows-msvc`
  - `x86_64-apple-darwin`
  - `aarch64-apple-darwin`

- #### Ubuntu(Other Linux)

Making [Releases](https://github.com/gngpp/ninja/releases/latest) has a precompiled deb package, binaries, in Ubuntu, for example:

```shell
wget https://github.com/gngpp/ninja/releases/download/v0.9.16/ninja-0.9.16-x86_64-unknown-linux-musl.tar.gz
tar -xf ninja-0.9.16-x86_64-unknown-linux-musl.tar.gz
mv ./ninja /bin/ninja
./ninja run

# Online update version
ninja update

# Run the process in the foreground
ninja run

# Run the process in the background
ninja start

# Stop background process
ninja stop

# Restart background process
ninja restart

# Check background process status
ninja status

# View background process logs
ninja log

# Generate configuration file template, serve.toml
ninja gt -o serve.toml

# Specify the configuration file template to run, bypassing the cumbersome cli commands
ninja (run/start/restart) -C serve.toml
```

- #### OpenWrt

There are pre-compiled ipk files in GitHub [Releases](https://github.com/gngpp/ninja/releases/latest), which currently provide versions of aarch64/x86_64 and other architectures. After downloading, use opkg to install, and use nanopi r4s as example:

```shell
wget https://github.com/gngpp/ninja/releases/download/v0.9.16/ninja_0.9.16_aarch64_generic.ipk
wget https://github.com/gngpp/ninja/releases/download/v0.9.16/luci-app-ninja_1.1.6-1_all.ipk
wget https://github.com/gngpp/ninja/releases/download/v0.9.16/luci-i18n-ninja-zh-cn_1.1.6-1_all.ipk

opkg install ninja_0.9.16_aarch64_generic.ipk
opkg install luci-app-ninja_1.1.6-1_all.ipk
opkg install luci-i18n-ninja-zh-cn_1.1.6-1_all.ipk
```

- #### Docker

> Mirror source supports `gngpp/ninja:latest`/`ghcr.io/gngpp/ninja:latest`

```shell
docker run --rm -it -p 7999:7999 --name=ninja \
  -e LOG=info \
  ghcr.io/gngpp/ninja:latest run
```

- Docker Compose

> `CloudFlare Warp` is not supported in your region (China), please delete it, or if your `VPS` IP can be directly connected to `OpenAI`, you can also delete it

```yaml
version: '3'

services:
  ninja:
    image: gngpp/ninja:latest
    container_name: ninja
    restart: unless-stopped
    environment:
      - TZ=Asia/Shanghai
      - PROXIES=socks5://warp:10000
    command: run
    ports:
      - "8080:7999"
    depends_on:
      - warp

  warp:
    container_name: warp
    image: ghcr.io/gngpp/warp:latest
    restart: unless-stopped

  watchtower:
    container_name: watchtower
    image: containrrr/watchtower
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock
    command: --interval 3600 --cleanup
    restart: unless-stopped

```

### ArkoseLabs

Sending `GPT-4/GPT-3.5/Creating API-Key` dialog requires sending `Arkose Token` as a parameter.

1) Use HAR

- Support HAR feature pooling, multiple HAR can be uploaded simultaneously, using round-robin strategy. The following is the method to obtain the HAR file:
  - First, log in to the `ChatGPT` GPT4 question interface, press the `F12` key, and the browser console will open. Find `network` and click with the left mouse button (If your console is in Chinese, it will be displayed as `网络`), and the browser's network capture interface will switch to.
  - With the console open, send a `GPT-4` session message, then find `filter` in the capture interface (If your console is in Chinese, it will be displayed as `过滤`), enter this address for filtering: `https://tcr9i.chat.openai.com/fc/gt2/public_key/35536E1E-65B4-4D96-9D97-6ADB7EFF8147`
  - At least one record will be filtered out. Randomly select one and download the HAR log record file of this interface. The specific operation is: right-click on this record, then find `Save all as HAR with content` (If your console is in Chinese, it will be displayed as `以 HAR 格式保存所有内容`).
  - Use the startup parameter `--arkose-gpt4-har-dir` to specify the HAR directory path (if no path is specified, the default path `~/.ninja/gpt4` will be used), and directly upload and update the HAR. The same method applies to `GPT-3.5` and other types. Support WebUI for uploading and updating HAR, request path: `/har/upload`, optional authentication parameter: `--arkose-har-upload-key`.

2) Use [Fcsrv](https://github.com/gngpp/fcsrv) / [YesCaptcha](https://yescaptcha.com/i/1Cc5i4) / [CapSolver](https://dashboard.capsolver.com/passport/register?inviteCode=y7CtB_a-3X6d)


- `Fcsrv` / `YesCaptcha` / `CapSolver` is recommended to be used with HAR. When the verification code is generated, the parser is called for processing.

The platform performs verification code parsing, and the startup parameter `--arkose-solver` selects the platform (default uses `Fcsrv`), `--arkose-solver-key` fills in the `Client Key`, and selects the customized submission node URL, for example: `--arkose-solver-endpoint http://localhost:8000/task`, `Fcsrv`/`YesCaptcha`/`CapSolver` are supported, `Fcsrv`/`YesCaptcha`/`CapSolver` is supported, `Fcsrv`/`YesCaptcha`/`CapSolver` Everyone supports it. Say important things three times.

Currently OpenAI has updated `Login` which requires verification of `Arkose Token`. The solution is the same as `GPT-4`. Fill in the startup parameters and specify the HAR file `--arkose-auth-har-dir`. To create an API-Key, you need to upload the HAR feature file related to the Platform. The acquisition method is the same as above.

`OpenAI` cancels `Arkose` verification for `GPT-3.5` and can be used without uploading HAR feature files (uploaded ones will not be affected). After compatibility, `Arkose` verification may be turned on again, and startup parameters need to be added`-- arkose-gpt3-experiment` enables the `GPT-3.5` model `Arkose` verification process, and the WebUI is not affected. If you encounter `418 I'm a teapot`, you can enable `--arkose-gpt3-experiment`, and you need to upload `HAR` features. If there are no `GPT-3.5` features, `GPT-4` features are also required. It can be used. If it still doesn't work, try to enable `--arkose-gpt3-experiment-solver`, which may use a third-party platform to solve the verification code.

### Http Server

#### Public interface, `*` represents any `URL` suffix

- ChatGPT-API
  - `/public-api/*`
  - `/backend-api/*`
  
- OpenAI-API
  - `/v1/*`

- Platform-API
  - `/dashboard/*`

- ChatGPT-To-API
  - `/v1/chat/completions`
  > About using `ChatGPT` to `API`, use `AceessToken` directly as `API Key`

- Files-API
  - `/files/*`
  > Image and file upload and download API proxy, the API returned by the `/backend-api/files` interface has been converted to `/files/*`

- Arkose-API
  - `/auth/arkose_token/:pk`
  > where pk is the arkose type ID, such as requesting Arkose for GPT4, `/auth/arkose_token/35536E1E-65B4-4D96-9D97-6ADB7EFF8147`. If `GPT-4` starts to force blob parameters, you need to bring `AccessToken` - > `Authorization: Bearer xxxx`

- Authorization
  > Except for login, use `Authorization: Bearer xxxx`, [Python Example](https://github.com/gngpp/ninja/blob/main/doc/authorization.md)
  
  - Login: `POST /auth/token`, form `option` optional parameter, default is `web` login, returns `AccessToken` and `Session`; parameter is `apple`/`platform`, returns `AccessToken` and `RefreshToken`
  - Refresh `RefreshToken`: `POST /auth/refresh_token`, support `platform`/`apple` revocation
  - Revoke `RefreshToken`: `POST /auth/revoke_token`, supports `platform`/`apple` revocation
  - Refresh `Session`: `POST /auth/refresh_session`, use the `Session` returned by `web` login to refresh
  - Obtain `Sess token`: `POST /auth/sess_token`, use `AccessToken` of `platform` to obtain
  - Obtain `Billing`: `GET /auth/billing`, use `sess token` to obtain
  
  `Web login`, a cookie named: `__Secure-next-auth.session-token` is returned by default. The client only needs to save this cookie. Calling `/auth/refresh_session` can also refresh `AccessToken`

  About the method of obtaining `RefreshToken`, use the `ChatGPT App` login method of the `Apple` platform. The principle is to use the built-in MITM agent. When the `Apple device` is connected to the agent, you can log in to the `Apple platform` to obtain `RefreshToken`. It is only suitable for small quantities or personal use `(large quantities will seal the device, use with caution)`. For detailed usage, please see the startup parameter description.

  ```shell
  # Generate certificate
  ninja genca

  ninja run --pbind 0.0.0.0:8888

  # Set the network on your mobile phone to set your proxy listening address, for example: http://192.168.1.1:8888
  # Then open the browser http://192.168.1.1:8888/preauth/cert, download the certificate, install it and trust it, then open iOS ChatGPT and you can play happily
   ```

#### API documentation

- Platfrom API [doc](https://platform.openai.com/docs/api-reference)
- Backend API [doc1](https://github.com/gngpp/ninja/blob/main/doc/rest.http) or [doc2](https://github.com/gngpp/ninja/blob/main/doc/python-requests.ipynb)
  > The example is only part of it, the official `API` is proxied according to `/backend-api/*`

#### Basic services

- ChatGPT WebUI
- Expose `ChatGPT-API`/`OpenAI-API` proxies
- `API` prefix is consistent with the official one
- `ChatGPT` to `API`
- Can access third-party clients
- Can access IP proxy pool to improve concurrency
- Supports obtaining RefreshToken
- Support file feature pooling in HAR format

#### Parameter Description

- `--level`, environment variable `LOG`, log level: default info
- `--bind`, environment variable `BIND`, service listening address: default 0.0.0.0:7999,
- `--tls-cert`, environment variable `TLS_CERT`', TLS certificate public key. Supported format: EC/PKCS8/RSA
- `--tls-key`, environment variable `TLS_KEY`, TLS certificate private key
- `--disable-webui`, if you don’t want to use the default built-in WebUI, use this parameter to turn it off
- `--enable-file-proxy`, environment variable `ENABLE_FILE_PROXY`, turns on the file upload and download API proxy
- `--enable-arkose-proxy`, enable obtaining `Arkose Token` endpoint
- `--enable-direct`, enable direct connection, add the IP bound to the `interface` export to the proxy pool
- `--proxies`, proxy, supports proxy pool, multiple proxies are separated by `,`, format: protocol://user:pass@ip:port
- `--no-keepalive` turns off Http Client Tcp keepalive
- `--fastest-dns` Use the built-in fastest DNS group
- `--visitor-email-whitelist`, whitelist restriction, the restriction is for AccessToken, the parameter is the email address, multiple email addresses are separated by `,`
- `--cookie-store`, enable Cookie Store
- `--cf-site-key`, Cloudflare turnstile captcha site key
- `--cf-secret-key`, Cloudflare turnstile captcha secret key
- `--arkose-endpoint`, ArkoseLabs endpoint, for example: <https://client-api.arkoselabs.com>
- `--arkose-solver`, ArkoseLabs solver platform, for example: yescaptcha
- `--arkose-solver-key`, ArkoseLabs solver client key
- `--arkose-gpt3-experiment`, to enable GPT-3.5 ArkoseLabs experiment
- `--arkose-gpt3-experiment-solver`, to open the GPT-3.5 ArkoseLabs experiment, you need to upload the HAR feature file, and the correctness of the ArkoseToken will be verified
- `--impersonate-uas`, you can optionally simulate UA randomly. Use `,` to separate multiple ones. Please see the command manual for details.
- `--auth-key`, login `API` authentication `Key`, sent with `Authorization Bearer` format

##### Advanced proxy usage

The built-in protocols and proxy types of agents are divided into built-in protocols: `all/api/auth/arkose`, where `all` is for all clients, `api` is for all `OpenAI API`, `auth` is for authorization/login, `arkose` For ArkoseLabs; proxy type: `interface/proxy/ipv6_subnet`, where `interface` represents the bound export `IP` address, `proxy` represents the upstream proxy protocol: `http/https/socks5/socks5h`, `ipv6_subnet` represents the A random IP address within the IPv6 subnet acts as a proxy. The format is `proto|proxy`, example: **`all|socks5://192.168.1.1:1080, api|10.0.0.1, auth|2001:db8::/32, http://192.168.1.1:1081`**, without built-in protocol, the protocol defaults to `all`.

> Regarding the proxy `http/https/socks5/socks5h`, only when the `socks5h` protocol is used, the DNS resolution will go through the proxy resolution, otherwise the `local`/`built-in` DNS resolution will be used

##### Agent usage rules

1) The existence of `interface` \ `proxy` \ `ipv6_subnet`

When `--enable-direct` is turned on, `proxy` + `interface` will be used as the proxy pool; if `--enable-direct` is not turned on, `proxy` will be used only if the number of `proxy` is greater than or equal to 2, otherwise it will Use `ipv6_subnet` as the proxy pool and `interface` as the fallback address.

2) The existence of `interface` \ `proxy`

When `--enable-direct` is turned on, `proxy` + `interface` will be used as the proxy pool; if `--enable-direct` is not turned on, only `proxy` will be used as the proxy pool.
  
3) The existence of `proxy` \ `ipv6_subnet`

The rules are the same as (1), except that there is no `interface` as the fallback address.

4) The existence of `interface` \ `ipv6_subnet`
When `--enable-direct` is turned on and the number of `interface` is greater than or equal to 2, `interface` will be used as the proxy pool; if `--enable-direct` is not turned on, `ipv6_subnet` will be used as the proxy pool and `interface` will be used as the proxy pool. fallback address.

5) The existence of `proxy`

When `--enable-direct` is enabled, `proxy` + default direct connection is used as the proxy pool; when `--enable-direct` is not enabled, only `proxy` is used as the proxy pool

6) The existence of `ipv6_subnet`

Regardless of whether `--enable-direct` is turned on, `ipv6_subnet` will be used as the proxy pool

### Command Manual

```shell
$ ninja --help
Reverse engineered ChatGPT proxy

Usage: ninja [COMMAND]

Commands:
  run      Run the HTTP server
  stop     Stop the HTTP server daemon
  start    Start the HTTP server daemon
  restart  Restart the HTTP server daemon
  status   Status of the Http server daemon process
  log      Show the Http server daemon log
  genca    Generate MITM CA certificate
  ua      Show the impersonate user-agent list
  gt       Generate config template file (toml format file)
  update   Update the application
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version

$ ninja run --help
Run the HTTP server

Usage: ninja run [OPTIONS]

Options:
  -L, --level <LEVEL>
          Log level (info/debug/warn/trace/error) [env: LOG=] [default: info]
  -C, --config <CONFIG>
          Configuration file path (toml format file) [env: CONFIG=]
  -b, --bind <BIND>
          Server bind address [env: BIND=] [default: 0.0.0.0:7999]
      --concurrent-limit <CONCURRENT_LIMIT>
          Server Enforces a limit on the concurrent number of requests the underlying [default: 1024]
      --timeout <TIMEOUT>
          Server/Client timeout (seconds) [default: 360]
      --connect-timeout <CONNECT_TIMEOUT>
          Server/Client connect timeout (seconds) [default: 5]
      --tcp-keepalive <TCP_KEEPALIVE>
          Server/Client TCP keepalive (seconds) [default: 60]
  -H, --no-keepalive
          No TCP keepalive (Client) [env: NO_TCP_KEEPALIVE=]
      --pool-idle-timeout <POOL_IDLE_TIMEOUT>
          Keep the client alive on an idle socket with an optional timeout set [default: 90]
  -x, --proxies <PROXIES>
          Client proxy, support multiple proxy, use ',' to separate, Format: proto|type
          Proto: all/api/auth/arkose, default: all
          Type: interface/proxy/ipv6 subnet，proxy type only support: socks5/http/https
          Example: all|socks5://192.168.1.1:1080, api|10.0.0.1, auth|2001:db8::/32, http://192.168.1.1:1081 [env: PROXIES=]
      --enable-direct
          Enable direct connection [env: ENABLE_DIRECT=]
  -I, --impersonate-uas <IMPERSONATE_UAS>
          Impersonate User-Agent, separate multiple ones with "," [env: IMPERSONATE_UA=]
      --cookie-store
          Enabled Cookie Store [env: COOKIE_STORE=]
      --fastest-dns
          Use fastest DNS resolver [env: FASTEST_DNS=]
      --tls-cert <TLS_CERT>
          TLS certificate file path [env: TLS_CERT=]
      --tls-key <TLS_KEY>
          TLS private key file path (EC/PKCS8/RSA) [env: TLS_KEY=]
      --cf-site-key <CF_SITE_KEY>
          Cloudflare turnstile captcha site key [env: CF_SECRET_KEY=]
      --cf-secret-key <CF_SECRET_KEY>
          Cloudflare turnstile captcha secret key [env: CF_SITE_KEY=]
  -A, --auth-key <AUTH_KEY>
          Login Authentication Key [env: AUTH_KEY=]
  -D, --disable-webui
          Disable WebUI [env: DISABLE_WEBUI=]
  -F, --enable-file-proxy
          Enable file endpoint proxy [env: ENABLE_FILE_PROXY=]
  -G, --enable-arkose-proxy
          Enable arkose token endpoint proxy [env: ENABLE_ARKOSE_PROXY=]
  -W, --visitor-email-whitelist <VISITOR_EMAIL_WHITELIST>
          Visitor email whitelist [env: VISITOR_EMAIL_WHITELIST=]
      --arkose-endpoint <ARKOSE_ENDPOINT>
          Arkose endpoint, Example: https://client-api.arkoselabs.com
  -E, --arkose-gpt3-experiment
          Enable Arkose GPT-3.5 experiment
  -S, --arkose-gpt3-experiment-solver
          Enable Arkose GPT-3.5 experiment solver
      --arkose-gpt3-har-dir <ARKOSE_GPT3_HAR_DIR>
          About the browser HAR directory path requested by ChatGPT GPT-3.5 ArkoseLabs
      --arkose-gpt4-har-dir <ARKOSE_GPT4_HAR_DIR>
          About the browser HAR directory path requested by ChatGPT GPT-4 ArkoseLabs
      --arkose-auth-har-dir <ARKOSE_AUTH_HAR_DIR>
          About the browser HAR directory path requested by Auth ArkoseLabs
      --arkose-platform-har-dir <ARKOSE_PLATFORM_HAR_DIR>
          About the browser HAR directory path requested by Platform ArkoseLabs
  -K, --arkose-har-upload-key <ARKOSE_HAR_UPLOAD_KEY>
          HAR file upload authenticate key
  -s, --arkose-solver <ARKOSE_SOLVER>
          About ArkoseLabs solver platform [default: fcsrv]
  -k, --arkose-solver-key <ARKOSE_SOLVER_KEY>
          About the solver client key by ArkoseLabs
      --arkose-solver-endpoint <ARKOSE_SOLVER_ENDPOINT>
          About the solver client endpoint by ArkoseLabs
      --arkose-solver-limit <ARKOSE_SOLVER_LIMIT>
          About the solver submit multiple image limit by ArkoseLabs [default: 1]
      --arkose-solver-tguess-endpoint <ARKOSE_SOLVER_TGUESS_ENDPOINT>
          About the solver tguess endpoint by ArkoseLabs
  -T, --tb-enable
          Enable token bucket flow limitation
      --tb-strategy <TB_STRATEGY>
          Token bucket store strategy (mem/redb) [default: mem]
      --tb-capacity <TB_CAPACITY>
          Token bucket capacity [default: 60]
      --tb-fill-rate <TB_FILL_RATE>
          Token bucket fill rate [default: 1]
      --tb-expired <TB_EXPIRED>
          Token bucket expired (seconds) [default: 86400]
  -B, --pbind <PBIND>
          Preauth MITM server bind address [env: PREAUTH_BIND=]
  -X, --pupstream <PUPSTREAM>
          Preauth MITM server upstream proxy
          Supports: http/https/socks5/socks5h [env: PREAUTH_UPSTREAM=]
      --pcert <PCERT>
          Preauth MITM server CA certificate file path [default: ca/cert.crt]
      --pkey <PKEY>
          Preauth MITM server CA private key file path [default: ca/key.pem]
  -h, --help
          Print help
```

### Compile

- Linux compile, Ubuntu machine for example:

```shell
apt install build-essential
apt install cmake
apt install libclang-dev

git clone https://github.com/gngpp/ninja.git && cd ninja
cargo build --release
```

- OpenWrt Compile

```shell
cd package
svn co https://github.com/gngpp/ninja/trunk/openwrt
cd -
make menuconfig # choose LUCI->Applications->luci-app-ninja  
make V=s
```
