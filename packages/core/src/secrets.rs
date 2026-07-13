//! Secrets flow — T6.3. Per MASTER_PROMPT.md pillar 6: "secrets stored in
//! the OS keychain and written to tool configs as env placeholders only."
//!
//! The flow has two halves:
//!
//! * **Harvest** — when an import finds a real secret value in a tool's
//!   config (an MCP server's `env` block, say), the value goes into the
//!   [`SecretStore`] under a key namespaced by server + variable, and the
//!   config side keeps only a `${VAR_NAME}` placeholder. One stored
//!   secret then serves every tool the server is projected to
//!   ("project-to-all-tools") — the placeholder is what all 12 adapters
//!   write; no tool config ever holds the value again.
//! * **Resolve** — when something needs the real value back (spawning a
//!   server locally, a health check with credentials), placeholders are
//!   looked up in the store.
//!
//! [`SecretStore`] is a trait so the storage is pluggable:
//! [`MemorySecretStore`] is the fixture backend every test uses (T6.3's
//! verify text is literally "secret fixture round-trip");
//! `OsKeychainStore` (behind the `os-keychain` cargo feature) is the real
//! backend — Secret Service on Linux, Security.framework on macOS,
//! Credential Manager on Windows — which headless CI can't exercise
//! because there is no unlocked keyring to talk to.

use std::collections::BTreeMap;

#[derive(Debug, PartialEq, Eq)]
pub enum SecretError {
    NotFound(String),
    Backend(String),
}

impl std::fmt::Display for SecretError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecretError::NotFound(key) => write!(f, "no secret stored for: {key}"),
            SecretError::Backend(msg) => write!(f, "secret backend error: {msg}"),
        }
    }
}

impl std::error::Error for SecretError {}

/// Pluggable secret storage. Keys are opaque strings; use
/// [`secret_key`] to build them consistently.
pub trait SecretStore {
    fn set(&mut self, key: &str, value: &str) -> Result<(), SecretError>;
    fn get(&self, key: &str) -> Result<String, SecretError>;
    fn delete(&mut self, key: &str) -> Result<(), SecretError>;
}

/// The canonical store key for one env var of one MCP server, e.g.
/// `mcp/github-mcp/GITHUB_TOKEN`. One key per (server, variable) — every
/// tool the server is projected to resolves through this same key.
pub fn secret_key(server_id: &str, var_name: &str) -> String {
    format!("mcp/{server_id}/{var_name}")
}

/// The placeholder written into tool configs in place of the value.
pub fn placeholder(var_name: &str) -> String {
    format!("${{{var_name}}}")
}

/// True if `value` is a placeholder rather than a real value — used to
/// avoid double-harvesting a config that was already redacted.
pub fn is_placeholder(value: &str) -> bool {
    value.starts_with("${") && value.ends_with('}')
}

/// Harvests real values out of an imported env map: each non-placeholder
/// value is stored under [`secret_key`], and the returned map carries
/// placeholders only — safe to hand to any adapter's `project()`.
/// Values that are already placeholders pass through untouched.
pub fn harvest_env(
    store: &mut dyn SecretStore,
    server_id: &str,
    env: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>, SecretError> {
    let mut redacted = BTreeMap::new();
    for (name, value) in env {
        if !is_placeholder(value) && !value.is_empty() {
            store.set(&secret_key(server_id, name), value)?;
        }
        redacted.insert(name.clone(), placeholder(name));
    }
    Ok(redacted)
}

/// The reverse of [`harvest_env`]: swaps placeholders back for real
/// values from the store. Non-placeholder entries pass through untouched;
/// a placeholder with no stored secret is an error (the caller was about
/// to launch something with a missing credential — fail loudly, not with
/// a literal `${...}` string in a child's environment).
pub fn resolve_env(
    store: &dyn SecretStore,
    server_id: &str,
    env: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>, SecretError> {
    let mut resolved = BTreeMap::new();
    for (name, value) in env {
        if is_placeholder(value) {
            resolved.insert(name.clone(), store.get(&secret_key(server_id, name))?);
        } else {
            resolved.insert(name.clone(), value.clone());
        }
    }
    Ok(resolved)
}

/// In-memory fixture backend — what every test (and any headless
/// environment) uses.
#[derive(Debug, Default)]
pub struct MemorySecretStore {
    entries: BTreeMap<String, String>,
}

impl SecretStore for MemorySecretStore {
    fn set(&mut self, key: &str, value: &str) -> Result<(), SecretError> {
        self.entries.insert(key.to_string(), value.to_string());
        Ok(())
    }

    fn get(&self, key: &str) -> Result<String, SecretError> {
        self.entries
            .get(key)
            .cloned()
            .ok_or_else(|| SecretError::NotFound(key.to_string()))
    }

    fn delete(&mut self, key: &str) -> Result<(), SecretError> {
        self.entries
            .remove(key)
            .map(|_| ())
            .ok_or_else(|| SecretError::NotFound(key.to_string()))
    }
}

/// The real OS keychain, behind the `os-keychain` feature. Same trait,
/// same round-trip contract as the fixture store.
#[cfg(feature = "os-keychain")]
pub struct OsKeychainStore {
    /// Keychain "service" name all entries live under.
    service: String,
}

#[cfg(feature = "os-keychain")]
impl OsKeychainStore {
    pub fn new(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
        }
    }

    fn entry(&self, key: &str) -> Result<keyring::Entry, SecretError> {
        keyring::Entry::new(&self.service, key).map_err(|e| SecretError::Backend(e.to_string()))
    }
}

#[cfg(feature = "os-keychain")]
impl SecretStore for OsKeychainStore {
    fn set(&mut self, key: &str, value: &str) -> Result<(), SecretError> {
        self.entry(key)?
            .set_password(value)
            .map_err(|e| SecretError::Backend(e.to_string()))
    }

    fn get(&self, key: &str) -> Result<String, SecretError> {
        match self.entry(key)?.get_password() {
            Ok(v) => Ok(v),
            Err(keyring::Error::NoEntry) => Err(SecretError::NotFound(key.to_string())),
            Err(e) => Err(SecretError::Backend(e.to_string())),
        }
    }

    fn delete(&mut self, key: &str) -> Result<(), SecretError> {
        match self.entry(key)?.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Err(SecretError::NotFound(key.to_string())),
            Err(e) => Err(SecretError::Backend(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_env() -> BTreeMap<String, String> {
        BTreeMap::from([
            (
                "GITHUB_TOKEN".to_string(),
                "ghp_real_secret_value".to_string(),
            ),
            ("API_URL".to_string(), "${API_URL}".to_string()), // already redacted
        ])
    }

    /// T6.3's verify condition: the secret fixture round-trip.
    /// Real value → harvested into the store, placeholder in the config →
    /// resolved back to the identical value.
    #[test]
    fn secret_fixture_round_trips_through_harvest_and_resolve() {
        let mut store = MemorySecretStore::default();
        let original = fixture_env();

        let redacted = harvest_env(&mut store, "github-mcp", &original).unwrap();

        // Config side holds placeholders only.
        assert_eq!(redacted["GITHUB_TOKEN"], "${GITHUB_TOKEN}");
        assert_eq!(redacted["API_URL"], "${API_URL}");
        assert!(!redacted
            .values()
            .any(|v| v.contains("ghp_real_secret_value")));

        // Store side holds the value under the namespaced key.
        assert_eq!(
            store
                .get(&secret_key("github-mcp", "GITHUB_TOKEN"))
                .unwrap(),
            "ghp_real_secret_value"
        );

        // Resolve restores the original value byte-for-byte.
        let resolved = resolve_env(&store, "github-mcp", &redacted);
        // API_URL was a placeholder with no stored value — must fail loudly.
        assert!(matches!(resolved, Err(SecretError::NotFound(_))));

        // Store the missing one, then the full round trip succeeds.
        store
            .set(
                &secret_key("github-mcp", "API_URL"),
                "https://api.github.com",
            )
            .unwrap();
        let resolved = resolve_env(&store, "github-mcp", &redacted).unwrap();
        assert_eq!(resolved["GITHUB_TOKEN"], "ghp_real_secret_value");
        assert_eq!(resolved["API_URL"], "https://api.github.com");
    }

    /// "Project-to-all-tools": one stored secret serves every tool's
    /// projected config — each tool gets the same placeholder, and each
    /// resolves to the same single stored value.
    #[test]
    fn one_stored_secret_serves_every_projected_tool() {
        let mut store = MemorySecretStore::default();
        let env = BTreeMap::from([("TOKEN".to_string(), "s3cret".to_string())]);

        let redacted = harvest_env(&mut store, "shared-server", &env).unwrap();

        for _tool in ["claude-code", "cursor", "cline", "zed"] {
            // Every adapter would write this same redacted map; resolving
            // from any of them hits the same store key.
            let resolved = resolve_env(&store, "shared-server", &redacted).unwrap();
            assert_eq!(resolved["TOKEN"], "s3cret");
        }
    }

    #[test]
    fn harvest_never_stores_placeholders_or_empties() {
        let mut store = MemorySecretStore::default();
        let env = BTreeMap::from([
            ("ALREADY".to_string(), "${ALREADY}".to_string()),
            ("EMPTY".to_string(), String::new()),
        ]);

        harvest_env(&mut store, "srv", &env).unwrap();

        assert!(store.get(&secret_key("srv", "ALREADY")).is_err());
        assert!(store.get(&secret_key("srv", "EMPTY")).is_err());
    }

    #[test]
    fn delete_removes_and_double_delete_errors() {
        let mut store = MemorySecretStore::default();
        store.set("k", "v").unwrap();
        store.delete("k").unwrap();
        assert_eq!(
            store.delete("k"),
            Err(SecretError::NotFound("k".to_string()))
        );
    }

    #[test]
    fn placeholder_detection() {
        assert!(is_placeholder("${TOKEN}"));
        assert!(!is_placeholder("plainvalue"));
        assert!(!is_placeholder("${unclosed"));
    }

    /// Real OS-keychain round trip — only compiled with the
    /// `os-keychain` feature, since it needs a live, unlocked keyring
    /// (Secret Service on Linux). Uses a clearly-namespaced test entry
    /// and deletes it before asserting, so nothing lingers in the user's
    /// keyring even on failure.
    #[cfg(feature = "os-keychain")]
    #[test]
    fn os_keychain_round_trips_a_real_secret() {
        let mut store = OsKeychainStore::new("llm-neurosurgeon-test");
        let key = "mcp/self-test/PROBE";

        if let Err(e) = store.set(key, "probe-value") {
            eprintln!("skipping: no usable OS keychain in this environment ({e})");
            return;
        }
        let fetched = store.get(key);
        let deleted = store.delete(key);

        assert_eq!(fetched.unwrap(), "probe-value");
        deleted.unwrap();
        assert!(matches!(store.get(key), Err(SecretError::NotFound(_))));
    }
}
