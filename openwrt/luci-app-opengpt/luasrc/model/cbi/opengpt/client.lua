local m, s

m = Map("opengpt", translate("OpenGPT"))
m.description = translate("<a>A reverse engineered unofficial ChatGPT proxy (bypass Cloudflare 403 Access Denied)</a> | <a href=\"https://github.com/gngpp/opengpt\" target=\"_blank\">Project GitHub URL</a>")

m:section(SimpleSection).template = "opengpt/opengpt_status"

s = m:section(TypedSection, "opengpt")
s.addremove = false
s.anonymous = true

o = s:option(Flag, "enabled", translate("Enabled"))
o.rmempty = false

o = s:option(Value, "proxies", translate("Proxies"), translate("Supports http/https/socks5, format: protocol://user:pass@ip:port"))

o = s:option(Value, "level", translate("Log Level"), translate("info/debug/warn/trace/error"))
o.default = "info"

o = s:option(Value, "host", translate("Host"), translate("Default listening address: 0.0.0.0"))
o.default = "0.0.0.0"
o.datatype = "ipaddr"

o = s:option(Value, "port", translate("Port"), translate("Default listening port: 7999"))
o.datatype = "and(port,min(1))"
o.default = "7999"
o.rmempty = false

o = s:option(Value, "workers", translate("Workers"), translate("Default 1 worker thread"))
o.default = "1"

o = s:option(Value, "timeout", translate("Timeout"), translate("Client timeout (secends), default 600 secends"))
o.default = "600"

o = s:option(Value, "connect_timeout", translate("Connect timeout"), translate("Client connect timeout (secends), default 60 secends"))
o.default = "60"

o = s:option(Value, "tcp_keepalive", translate("TCP Keep-Alive"), translate("Default 60 seconds"))
o.default = "60"

o = s:option(Value, "tls_cert", translate("TLS certificate file path"), translate("Certificate in DER format"))

o = s:option(Value, "tls_key", translate("TLS private key file path"), translate("Supports EC/PKCS8/RSA type formats"))

o = s:option(Value, "sign_secret_key", translate("API Signature Secret Key"), translate("Enable API signature"))

o = s:option(Flag, "tb_enable", translate("Enable Token Bucket Limit"))
o.rmempty = false

o = s:option(ListValue, "tb_store_strategy", translate("Token Bucket Storage Strategy"), translate("Token bucket storage strategy, mem/redis"))
o:value("mem", "mem");
o:value("redis", "redis");
o.default = "mem"

o = s:option(Value, "tb_redis_url", translate("Token bucket redis url(support cluster)"), translate("Example: redis://user:pass@ip:port"))
o.default = "redis://127.0.0.1:6379"

o = s:option(Value, "tb_capacity", translate("Token Bucket Capacity"), translate("Token bucket capacity, the default is 60"))
o.default = "60"

o = s:option(Value, "tb_fill_rate", translate("Token Bucket Fill Rate"), translate("Token bucket fill rate, the default is 1"))
o.default = "1"

o = s:option(Value, "tb_expired", translate("Token Bucket Expired"), translate("Token bucket expired time, the default is 86400 seconds"))
o.default = "86400"

return m
