use futures_util::StreamExt;
use url::{Host, Url};
use worker::{
    AbortController, AbortSignal, Error, Fetch, Headers, Method, Request, RequestInit,
    RequestRedirect, Result,
};

pub fn public_url(raw: &str) -> Result<Url> {
    let mut url = Url::parse(raw).map_err(|e| Error::RustError(e.to_string()))?;
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some_and(|p| p != 443)
    {
        return Err(Error::RustError("Source must be a public HTTPS URL".into()));
    }
    match url.host() {
        Some(Host::Domain(host))
            if host.contains('.')
                && !host.ends_with(".local")
                && !host.ends_with(".internal")
                && !host.ends_with(".localhost") => {}
        _ => {
            return Err(Error::RustError(
                "IP and internal source addresses are forbidden".into(),
            ))
        }
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

pub async fn get(raw: &str, max_bytes: usize) -> Result<Document> {
    let abort = AbortController::default();
    let signal = abort.signal();
    let operation = get_inner(raw, max_bytes, &signal);
    let timeout = worker::Delay::from(std::time::Duration::from_secs(25));
    futures_util::pin_mut!(operation, timeout);
    let result = match futures_util::future::select(operation, timeout).await {
        futures_util::future::Either::Left((result, _)) => result,
        futures_util::future::Either::Right(_) => Err(Error::RustError("Source timed out".into())),
    };
    abort.abort();
    result
}

async fn get_inner(raw: &str, max_bytes: usize, signal: &AbortSignal) -> Result<Document> {
    let started = js_sys::Date::now();
    let mut url = public_url(raw)?;
    for _ in 0..5 {
        let mut init = RequestInit::new();
        init.with_method(Method::Get)
            .with_redirect(RequestRedirect::Manual);
        let headers = Headers::new();
        headers.set(
            "User-Agent",
            "Fid/0.1 (technical briefing; bounded evidence collection)",
        )?;
        headers.set("Accept", "text/html, application/atom+xml, application/rss+xml, application/json, application/xml, text/plain")?;
        init.with_headers(headers);
        let request = Request::new_with_init(url.as_str(), &init)?;
        let fetcher = Fetch::Request(request);
        let mut response = fetcher.send_with_signal(signal).await?;
        if (300..400).contains(&response.status_code()) {
            let location = response
                .headers()
                .get("Location")?
                .ok_or_else(|| Error::RustError("Redirect without Location".into()))?;
            url = public_url(
                url.join(&location)
                    .map_err(|e| Error::RustError(e.to_string()))?
                    .as_str(),
            )?;
            continue;
        }
        if response.status_code() != 200 {
            return Err(Error::RustError(format!(
                "Source HTTP {}",
                response.status_code()
            )));
        }
        let content_type = response.headers().get("Content-Type")?.unwrap_or_default();
        if !["text/", "json", "xml"]
            .iter()
            .any(|kind| content_type.contains(kind))
        {
            return Err(Error::RustError(format!(
                "Unsupported evidence format: {content_type}"
            )));
        }
        let mut bytes = Vec::new();
        let mut stream = response.stream()?;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            if bytes.len() + chunk.len() > max_bytes {
                return Err(Error::RustError(
                    "Source exceeds evidence size limit".into(),
                ));
            }
            bytes.extend(chunk);
        }
        worker::console_log!(
            "fid_usage source_fetch bytes={} elapsed_ms={}",
            bytes.len(),
            js_sys::Date::now() - started
        );
        let text = String::from_utf8(bytes).map_err(|e| Error::RustError(e.to_string()))?;
        return Ok(Document {
            url: url.to_string(),
            text,
            content_type,
        });
    }
    Err(Error::RustError("Too many source redirects".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
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
