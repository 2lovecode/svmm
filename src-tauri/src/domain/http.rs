use std::time::Duration;

use crate::error::{AppError, AppResult};
use crate::storage::settings;

/// Normalize a user-entered proxy. Empty means “use the system proxy”.
/// A host:port without a scheme is treated as `http://`.
pub fn normalize_download_proxy(raw: Option<&str>) -> AppResult<Option<String>> {
    let Some(raw) = raw.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    let with_scheme = if raw.contains("://") {
        raw.to_string()
    } else {
        format!("http://{raw}")
    };
    let url = reqwest::Url::parse(&with_scheme).map_err(|_| {
        AppError::new("proxy_invalid", "下载代理地址无效").with_detail(proxy_detail(raw))
    })?;
    match url.scheme() {
        "http" | "https" | "socks5" | "socks5h" => Ok(Some(with_scheme)),
        _ => Err(
            AppError::new("proxy_invalid", "下载代理只支持 http、https、socks5")
                .with_detail(proxy_detail(raw)),
        ),
    }
}

pub fn download_percent(received: u64, total: Option<u64>) -> Option<u8> {
    let total = total.filter(|n| *n > 0)?;
    Some(((received.saturating_mul(100)) / total).min(100) as u8)
}

pub fn blocking_client(timeout: Duration) -> AppResult<reqwest::blocking::Client> {
    let settings = settings::load_settings()?;
    client_with_proxy(timeout, settings.download_proxy.as_deref())
}

pub fn client_with_proxy(
    timeout: Duration,
    proxy: Option<&str>,
) -> AppResult<reqwest::blocking::Client> {
    let mut builder = reqwest::blocking::Client::builder()
        .timeout(timeout)
        .redirect(reqwest::redirect::Policy::limited(10));
    if let Some(url) = normalize_download_proxy(proxy)? {
        let proxy = reqwest::Proxy::all(url).map_err(|e| {
            AppError::new("proxy_invalid", "无法使用该下载代理").with_detail(e.to_string())
        })?;
        builder = builder.no_proxy().proxy(proxy);
    }
    builder.build().map_err(|e| {
        AppError::new("http_client_failed", "无法创建下载客户端").with_detail(e.to_string())
    })
}

fn proxy_detail(raw: &str) -> String {
    if raw.contains('@') {
        "已隐藏凭据".to_string()
    } else {
        raw.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_proxy_is_system_default() {
        assert_eq!(normalize_download_proxy(None).unwrap(), None);
        assert_eq!(normalize_download_proxy(Some("  ")).unwrap(), None);
    }

    #[test]
    fn host_port_defaults_to_http() {
        assert_eq!(
            normalize_download_proxy(Some("127.0.0.1:7890")).unwrap(),
            Some("http://127.0.0.1:7890".to_string())
        );
    }

    #[test]
    fn keeps_explicit_schemes() {
        assert_eq!(
            normalize_download_proxy(Some("socks5://127.0.0.1:7891")).unwrap(),
            Some("socks5://127.0.0.1:7891".to_string())
        );
    }

    #[test]
    fn rejects_unsupported_scheme() {
        let err = normalize_download_proxy(Some("ftp://127.0.0.1:21")).unwrap_err();
        assert_eq!(err.code, "proxy_invalid");
    }

    #[test]
    fn percent_uses_content_length() {
        assert_eq!(download_percent(0, Some(200)), Some(0));
        assert_eq!(download_percent(50, Some(200)), Some(25));
        assert_eq!(download_percent(500, Some(200)), Some(100));
        assert_eq!(download_percent(10, None), None);
        assert_eq!(download_percent(10, Some(0)), None);
    }
}
