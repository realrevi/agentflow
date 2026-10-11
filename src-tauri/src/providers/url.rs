//! Base URL rules: HTTPS only, except plain HTTP to this machine (Ollama and other local
//! servers). No credentials in the URL. Endpoints are appended to the base path.

use reqwest::Url;

use super::error::ProviderError;

pub fn parse_base_url(raw: &str) -> Result<Url, ProviderError> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err(ProviderError::config("The provider has no base URL."));
    }
    let url = Url::parse(raw)
        .map_err(|err| ProviderError::config(format!("Invalid base URL \"{raw}\": {err}.")))?;
    if !url.username().is_empty() || url.password().is_some() {
        return Err(ProviderError::config(
            "The base URL must not contain a user name or password.",
        ));
    }
    if url.fragment().is_some() {
        return Err(ProviderError::config(
            "The base URL must not contain a #fragment.",
        ));
    }
    match url.scheme() {
        "https" if url.host_str().is_some() => Ok(url),
        "http" if is_local(&url) => Ok(url),
        "http" => Err(ProviderError::config(
            "Use https:// for remote providers (plain http:// is only allowed for localhost).",
        )),
        other => Err(ProviderError::config(format!(
            "Unsupported URL scheme \"{other}\": use https:// (or http:// for localhost)."
        ))),
    }
}

fn is_local(url: &Url) -> bool {
    let Some(host) = url.host_str() else {
        return false;
    };
    let host = host.trim_start_matches('[').trim_end_matches(']');
    match host.parse::<std::net::IpAddr>() {
        Ok(ip) => ip.is_loopback(),
        Err(_) => {
            let name = host.to_ascii_lowercase();
            name == "localhost" || name.ends_with(".localhost")
        }
    }
}

/// `base` + `/` + `path`, keeping the base's path prefix (e.g. `/v1`) and query.
pub fn endpoint(base: &Url, path: &str) -> Url {
    let mut url = base.clone();
    let joined = format!(
        "{}/{}",
        base.path().trim_end_matches('/'),
        path.trim_start_matches('/')
    );
    url.set_path(&joined);
    url
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn https_everywhere_http_only_on_this_machine() {
        assert!(parse_base_url("https://api.openai.com/v1").is_ok());
        assert!(parse_base_url("  https://api.anthropic.com  ").is_ok());
        for local in [
            "http://localhost:11434/v1",
            "http://127.0.0.1:1234/v1",
            "http://[::1]:8080",
            "http://ollama.localhost/v1",
            "http://LOCALHOST:11434",
        ] {
            assert!(parse_base_url(local).is_ok(), "{local}");
        }
        for bad in [
            "http://api.openai.com/v1",
            "http://192.168.1.20:11434/v1",
            "http://localhost.evil.com/v1",
            "ftp://example.com",
            "file:///etc/passwd",
            "https://user:pass@example.com",
            "https://example.com/#x",
            "api.openai.com/v1",
            "",
        ] {
            assert!(parse_base_url(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn endpoints_keep_the_base_path() {
        let base = parse_base_url("https://api.groq.com/openai/v1/").expect("url");
        assert_eq!(
            endpoint(&base, "chat/completions").as_str(),
            "https://api.groq.com/openai/v1/chat/completions"
        );
        let root = parse_base_url("https://api.anthropic.com").expect("url");
        assert_eq!(
            endpoint(&root, "/v1/messages").as_str(),
            "https://api.anthropic.com/v1/messages"
        );
        let query = parse_base_url("https://example.com/openai?api-version=1").expect("url");
        assert_eq!(
            endpoint(&query, "models").as_str(),
            "https://example.com/openai/models?api-version=1"
        );
    }
}
