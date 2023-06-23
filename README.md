# opengpt
Not just a command-line ChatGPT (bypass Cloudflare 403 Access Denied)

### Features
- API-Key authentication
- Email/Password account authentication
- Forward Http API Proxy

### Compiler
```shell
git clone https://github.com/gngpp/opengpt.git && cd opengpt
./build.sh

# Cross-platform compilation (currently only supports x86_64/aarch64, does not mean that only x86_64/aarch64 can be compiled)
./corss-build.sh
```

### Http Server

- Support unofficial/official forward proxy forwarding
- The API prefix is the same as the official one, just change the hostname
> - official https://platform.openai.com/docs/api-reference
> - unofficial [doc](doc/rest.http)

```shell
$ opengpt --help
Start the http server

Usage: opengpt serve [OPTIONS]

Options:
      --debug
          Enable debug [env: OPENGPT_DEBUG=]
  -H, --host <HOST>
          Server Listen host [env: OPENGPT_HOST=] [default: 0.0.0.0]
  -P, --port <PORT>
          Server Listen port [env: OPENGPT_PORT=] [default: 7999]
  -W, --workers <WORKERS>
          Server worker-pool size (Recommended number of CPU cores) [env: OPENGPT_WORKERS=] [default: 1]
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

### Reference
- https://github.com/tjardoo/openai-client
- https://github.com/jpopesculian/reqwest-eventsource
