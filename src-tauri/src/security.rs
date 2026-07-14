use keyring::Entry;
use regex::Regex;
use std::net::IpAddr;
use url::Url;
use zeroize::Zeroizing;

const KEYRING_SERVICE: &str = "com.agenticcouncil.desktop";

#[derive(Clone, Default)]
pub struct CredentialLedger;

impl CredentialLedger {
    fn entry(provider_id: &str) -> Result<Entry, String> {
        if !provider_id
            .chars()
            .all(|value| value.is_ascii_alphanumeric() || matches!(value, '_' | '-'))
        {
            return Err("Invalid provider identifier.".into());
        }
        Entry::new(KEYRING_SERVICE, provider_id).map_err(|error| sanitize_error(&error.to_string()))
    }

    pub fn set(&self, provider_id: &str, value: String) -> Result<(), String> {
        if value.trim().is_empty() || value.len() > 16_384 {
            return Err("Credential must contain between 1 and 16,384 characters.".into());
        }
        let secret = Zeroizing::new(value);
        Self::entry(provider_id)?
            .set_password(secret.as_str())
            .map_err(|error| sanitize_error(&error.to_string()))
    }

    pub fn get(&self, provider_id: &str) -> Result<Zeroizing<String>, String> {
        Self::entry(provider_id)?
            .get_password()
            .map(Zeroizing::new)
            .map_err(|error| sanitize_error(&error.to_string()))
    }

    pub fn exists(&self, provider_id: &str) -> bool {
        Self::entry(provider_id)
            .and_then(|entry| entry.get_password().map_err(|error| error.to_string()))
            .is_ok()
    }

    pub fn delete(&self, provider_id: &str) -> Result<(), String> {
        let entry = Self::entry(provider_id)?;
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(sanitize_error(&error.to_string())),
        }
    }
}

pub fn validate_endpoint(raw: &str) -> Result<Url, String> {
    let url = Url::parse(raw).map_err(|_| "Provider endpoint is not a valid URL.".to_string())?;
    if url.username() != "" || url.password().is_some() || url.fragment().is_some() {
        return Err("Provider endpoints cannot contain credentials or URL fragments.".into());
    }
    let host = url
        .host_str()
        .ok_or_else(|| "Provider endpoint is missing a host.".to_string())?;
    let loopback = host.eq_ignore_ascii_case("localhost")
        || host.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback());
    if url.scheme() != "https" && !(url.scheme() == "http" && loopback) {
        return Err("Provider endpoints must use HTTPS. HTTP is permitted only for loopback inference servers.".into());
    }
    if let Ok(ip) = host.parse::<IpAddr>() {
        let forbidden = match ip {
            IpAddr::V4(value) => {
                value.is_unspecified()
                    || value.is_multicast()
                    || value.is_link_local()
                    || value.octets() == [169, 254, 169, 254]
            }
            IpAddr::V6(value) => value.is_unspecified() || value.is_multicast(),
        };
        if forbidden {
            return Err(
                "Link-local, metadata, multicast, and unspecified endpoints are blocked.".into(),
            );
        }
    }
    Ok(url)
}

pub fn sanitize_error(input: &str) -> String {
    let bearer = Regex::new(r"(?i)bearer\s+[A-Za-z0-9._~+/=-]{8,}").expect("constant regex");
    let common_key =
        Regex::new(r"(?i)(?:sk|key|api)[-_][A-Za-z0-9._~+/=-]{8,}").expect("constant regex");
    let query_key = Regex::new(r"(?i)([?&](?:key|api_key|token)=)[^&\s]+").expect("constant regex");
    let value = bearer.replace_all(input, "Bearer ****…****");
    let value = common_key.replace_all(&value, "sk-****…****");
    query_key.replace_all(&value, "$1****…****").into_owned()
}

pub fn sanitize_prompt_for_log(prompt: &str) -> String {
    let prefix: String = prompt.chars().take(50).collect();
    if prompt.chars().count() > 50 {
        format!("{prefix} [REDACTED]")
    } else {
        prefix
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_common_secret_shapes() {
        let text =
            sanitize_error("Authorization: Bearer abcdefghijklmnop and sk-exampleSECRET123?x=1");
        assert!(!text.contains("abcdefghijklmnop"));
        assert!(!text.contains("exampleSECRET123"));
    }

    #[test]
    fn endpoint_policy_rejects_insecure_remote_hosts() {
        assert!(validate_endpoint("http://example.com/v1").is_err());
        assert!(validate_endpoint("http://127.0.0.1:11434/v1").is_ok());
        assert!(validate_endpoint("https://api.example.com/v1").is_ok());
    }
}
