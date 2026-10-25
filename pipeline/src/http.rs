use crate::{store::error, Result};
use futures_util::StreamExt;
use std::{
    net::{IpAddr, SocketAddr},
    time::Duration,
};
use url::{Host, Url};

pub fn public_url(raw: &str) -> Result<Url> {
    let mut url = Url::parse(raw).map_err(|e| error(e.to_string()))?;
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some_and(|p| p != 443)
    {
        return Err(error("Source must be a public HTTPS URL"));
    }
    match url.host() {
        Some(Host::Domain(host))
            if host.contains('.')
                && !host.ends_with(".local")
                && !host.ends_with(".internal")
                && !host.ends_with(".localhost") => {}
        _ => return Err(error("IP and internal source addresses are forbidden")),
    }
    url.set_fragment(None);
    let pairs: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(k, _)| !k.starts_with("utm_") && !["fbclid", "gclid"].contains(&k.as_ref()))
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    url.set_query(None);
    if !pairs.is_empty() {
        url.query_pairs_mut().extend_pairs(pairs);
    }
    Ok(url)
}

pub struct Document {
    pub url: String,
    pub text: String,
    pub content_type: String,
}

fn public_address(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(ip) => {
            !ip.is_private()
                && !ip.is_loopback()
                && !ip.is_link_local()
                && !ip.is_unspecified()
                && !ip.is_multicast()
                && !ip.is_broadcast()
                && !ip.is_documentation()
                && ip.octets()[0] != 0
                && ip.octets()[0] < 240
                && !(ip.octets()[0] == 100 && (64..128).contains(&ip.octets()[1]))
        }
        IpAddr::V6(ip) => {
            if let Some(ip) = ip.to_ipv4_mapped() {
                return public_address(IpAddr::V4(ip));
            }
            !ip.is_loopback()
                && !ip.is_unspecified()
                && !ip.is_unique_local()
                && !ip.is_unicast_link_local()
                && !ip.is_multicast()
        }
    }
}

async fn client(url: &Url, timeout: Duration) -> Result<reqwest::Client> {
    let host = url.host_str().ok_or_else(|| error("URL has no host"))?;
    let addresses: Vec<SocketAddr> = tokio::net::lookup_host((host, 443)).await?.collect();
    if addresses.is_empty() || addresses.iter().any(|a| !public_address(a.ip())) {
        return Err(error("Host resolves to a nonpublic address"));
    }
    // Pin checked DNS results so validation and connection cannot resolve differently.
    Ok(reqwest::Client::builder()
        .https_only(true)
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(timeout)
        .resolve_to_addrs(host, &addresses)
        .user_agent("Fid/0.1 (technical briefing; bounded evidence collection)")
        .build()?)
}

async fn read(response: reqwest::Response, max_bytes: usize) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if bytes.len() + chunk.len() > max_bytes {
            return Err(error("Response exceeds size limit"));
        }
        bytes.extend(chunk);
    }
    Ok(bytes)
}

pub async fn get(raw: &str, max_bytes: usize) -> Result<Document> {
    tokio::time::timeout(Duration::from_secs(25), get_inner(raw, max_bytes)).await?
}

async fn get_inner(raw: &str, max_bytes: usize) -> Result<Document> {
    let mut url = public_url(raw)?;
    for _ in 0..5 {
        let response = client(&url, Duration::from_secs(25)).await?.get(url.clone())
            .header("Accept", "text/html, application/atom+xml, application/rss+xml, application/json, application/xml, text/plain")
            .send().await?;
        if response.status().is_redirection() {
            let location = response
                .headers()
                .get("Location")
                .ok_or_else(|| error("Redirect without Location"))?
                .to_str()
                .map_err(error)?;
            url = public_url(url.join(location).map_err(error)?.as_str())?;
            continue;
        }
        if response.status() != 200 {
            return Err(error(format!("Source HTTP {}", response.status())));
        }
        let content_type = response
            .headers()
            .get("Content-Type")
            .map(|v| v.to_str())
            .transpose()
            .map_err(error)?
            .unwrap_or_default()
            .to_string();
        if !["text/", "json", "xml"]
            .iter()
            .any(|kind| content_type.contains(kind))
        {
            return Err(error(format!(
                "Unsupported evidence format: {content_type}"
            )));
        }
        let text = String::from_utf8(read(response, max_bytes).await?).map_err(error)?;
        return Ok(Document {
            url: url.to_string(),
            text,
            content_type,
        });
    }
    Err(error("Too many source redirects"))
}

pub struct ProviderResponse {
    pub status: u16,
    pub body: Option<serde_json::Value>,
}

pub async fn post_json(url: &Url, key: &str, body: &serde_json::Value) -> Result<ProviderResponse> {
    tokio::time::timeout(Duration::from_secs(240), async {
        let response = client(url, Duration::from_secs(240))
            .await?
            .post(url.clone())
            .bearer_auth(key)
            .json(body)
            .send()
            .await?;
        let status = response.status().as_u16();
        // Provider errors may echo private content; never log or persist their bodies.
        let body = if status == 200 {
            Some(serde_json::from_slice(&read(response, 4_000_000).await?)?)
        } else {
            None
        };
        Ok::<_, crate::Error>(ProviderResponse { status, body })
    })
    .await?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resolved_local_and_metadata_addresses_are_rejected() {
        for address in [
            "127.0.0.1",
            "10.0.0.1",
            "169.254.169.254",
            "192.168.1.1",
            "100.64.0.1",
            "::1",
            "fc00::1",
            "fe80::1",
            "::ffff:127.0.0.1",
        ] {
            assert!(!public_address(address.parse().unwrap()), "{address}");
        }
        for address in ["1.1.1.1", "2606:4700:4700::1111"] {
            assert!(public_address(address.parse().unwrap()), "{address}");
        }
    }
    #[test]
    fn urls_are_public_and_tracking_is_removed() {
        for value in [
            "http://example.com",
            "https://127.0.0.1/a",
            "https://[::1]/",
            "https://user@example.com/",
            "https://server.local/a",
            "https://example.com:8080/",
        ] {
            assert!(public_url(value).is_err(), "{value}");
        }
        assert_eq!(
            public_url("https://example.com/post?v=2&utm_source=hn#x")
                .unwrap()
                .as_str(),
            "https://example.com/post?v=2"
        );
    }
}
