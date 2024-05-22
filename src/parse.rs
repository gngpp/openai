use anyhow::Context;
use openai::proxy;
use std::net::IpAddr;
use std::path::PathBuf;
use std::str::FromStr;

// parse socket address
pub fn parse_socket_addr(s: &str) -> anyhow::Result<std::net::SocketAddr> {
    let addr = s
        .parse::<std::net::SocketAddr>()
        .map_err(|_| anyhow::anyhow!(format!("`{}` isn't a socket address", s)))?;
    Ok(addr)
}

// url parse
pub fn parse_url(s: &str) -> anyhow::Result<String> {
    let url = url::Url::parse(s)
        .context("The Proxy Url format must be `protocol://user:pass@ip:port`")?;
    let protocol = url.scheme().to_string();
    match protocol.as_str() {
        "http" | "https" | "socks5" | "redis" | "rediss" => Ok(s.to_string()),
        _ => anyhow::bail!("Unsupported protocol: {}", protocol),
    }
}
// websocket url parse
pub fn parse_websocket_url(s: &str) -> anyhow::Result<String> {
    let url = url::Url::parse(s)
        .context("The WebSocket Proxy Url format must be `ws[s]://user:pass@ip:port`")?;
    let protocol = url.scheme().to_string();
    match protocol.as_str() {
        "wss" | "ws"   => Ok(s.to_string()),
        _ => anyhow::bail!("Unsupported protocol: {}", protocol),
    }
}

// proxy proto, format: proto|type, support proto: all/api/auth/arkose, support type: ip/url/cidr
pub fn parse_proxies_url(s: &str) -> anyhow::Result<Vec<proxy::Proxy>> {
    let split = s.split(',');
    let mut proxies: Vec<_> = vec![];

    for ele in split {
        let parts: Vec<_> = ele.split('|').collect();
        let (proto, typer) = if parts.len() != 2 {
            ("all", ele.trim())
        } else {
            (parts[0].trim(), parts[1].trim())
        };
        match (
            typer.parse::<IpAddr>(),
            url::Url::parse(typer),
            typer.parse::<cidr::Ipv6Cidr>(),
        ) {
            (Ok(ip_addr), _, _) => proxies.push(proxy::Proxy::try_from((proto, ip_addr))?),
            (_, Ok(url), _) => proxies.push(proxy::Proxy::try_from((proto, url))?),
            (_, _, Ok(cidr)) => proxies.push(proxy::Proxy::try_from((proto, cidr))?),
            _ => anyhow::bail!("Invalid proxy format: {}", typer),
        }
    }

    Ok(proxies)
}

/// parse file path
pub fn parse_file_path(s: &str) -> anyhow::Result<PathBuf> {
    let path =
        PathBuf::from_str(s).map_err(|_| anyhow::anyhow!(format!("`{}` isn't a path", s)))?;

    if !path.exists() {
        anyhow::bail!(format!("Path {} not exists", path.display()))
    }

    if !path.is_file() {
        anyhow::bail!(format!("{} not a file", path.display()))
    }

    Ok(path)
}

// parse directory path
pub fn parse_dir_path(s: &str) -> anyhow::Result<PathBuf> {
    let path =
        PathBuf::from_str(s).map_err(|_| anyhow::anyhow!(format!("`{}` isn't a path", s)))?;

    if !path.exists() {
        anyhow::bail!(format!("Path {} not exists", path.display()))
    }

    if !path.is_dir() {
        anyhow::bail!(format!("{} not a directory", path.display()))
    }

    Ok(path)
}

/// parse email whitelist
/// format: email1,email2,email3
pub fn parse_email_whitelist(s: &str) -> anyhow::Result<Vec<String>> {
    let split = s.split(',');
    let mut emails: Vec<_> = vec![];

    for ele in split {
        let email = ele.trim();
        if email.is_empty() {
            continue;
        }

        if email.contains('@') {
            emails.push(email.to_string());
        } else {
            anyhow::bail!("Invalid email format: {}", email)
        }
    }

    Ok(emails)
}

// parse impersonate user-agent
pub fn parse_impersonate_uas(s: &str) -> anyhow::Result<Vec<String>> {
    let split = s.split(',');
    let mut uas: Vec<_> = vec![];

    for ele in split {
        let ua = ele.trim();
        if ua.is_empty() {
            continue;
        }

        uas.push(ua.to_string());
    }

    Ok(uas)
}
