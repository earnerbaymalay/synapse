//! Auto-update channel — T7.3. Per MASTER_PROMPT.md's Phase 7: "app
//! auto-update (Tauri updater, signed)"; self-verify: "update channel
//! dry-run works."
//!
//! ## Honest scope
//! A *shipping* auto-updater needs a code-signing keypair (Tauri's updater
//! verifies a minisign/ed25519 signature over every downloaded artifact).
//! No such keypair exists yet — generating and safeguarding release keys is
//! Phase 8 (Package & Release) work, gated behind GATE 4. So this module
//! implements the half that can be built and *fully tested* now without
//! keys or a network round-trip:
//!
//!   * parse a release manifest (the JSON a channel endpoint serves),
//!   * select the newest release for a channel + target platform,
//!   * compare it against the running version with real semver ordering,
//!   * decide what an update pass *would* do — **without downloading or
//!     installing anything** (that is the "dry-run").
//!
//! Signature verification is represented explicitly rather than faked: a
//! decision carries the signature blob it *would* verify, and
//! [`verification_status`] returns [`SigningStatus::NotConfigured`] until a
//! real public key is wired in. The dry-run never reports an update as
//! "verified" — it reports "verification pending signing keys", so nobody
//! mistakes this for a trust boundary that already exists.

use semver::Version;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A release channel. `Stable` only ever offers final releases; `Beta`
/// additionally offers pre-releases (and still sees stable ones, choosing
/// whichever is newest).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    Stable,
    Beta,
}

impl Channel {
    /// Parses a channel name case-insensitively. Unknown names are an error
    /// rather than a silent default, so a typo can't quietly move a user
    /// onto the wrong channel.
    pub fn parse(s: &str) -> Result<Channel, UpdateError> {
        match s.trim().to_ascii_lowercase().as_str() {
            "stable" | "release" | "latest" => Ok(Channel::Stable),
            "beta" | "pre" | "prerelease" => Ok(Channel::Beta),
            other => Err(UpdateError::UnknownChannel(other.to_string())),
        }
    }

    fn accepts(&self, v: &Version) -> bool {
        match self {
            // Stable ignores any release with a pre-release tag (e.g. -beta.1).
            Channel::Stable => v.pre.is_empty(),
            // Beta accepts everything and lets version ordering pick the newest.
            Channel::Beta => true,
        }
    }
}

/// A downloadable artifact for one target platform.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformAsset {
    /// Where the artifact would be fetched from. Never fetched in a dry-run.
    pub url: String,
    /// Detached signature over the artifact (base64). Carried through the
    /// dry-run but only *verified* once a public key is configured.
    #[serde(default)]
    pub signature: String,
}

/// One release the channel offers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Release {
    pub version: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub pub_date: String,
    /// Per-platform assets, keyed by target triple-ish id (e.g.
    /// `"linux-x86_64"`, `"darwin-aarch64"`, `"windows-x86_64"`).
    #[serde(default)]
    pub platforms: HashMap<String, PlatformAsset>,
}

/// The document a channel endpoint serves (or a bundled fixture).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseManifest {
    pub channel: Channel,
    #[serde(default)]
    pub releases: Vec<Release>,
}

impl ReleaseManifest {
    /// Parses a manifest from JSON.
    pub fn from_json(raw: &str) -> Result<ReleaseManifest, UpdateError> {
        serde_json::from_str(raw).map_err(|e| UpdateError::Manifest(e.to_string()))
    }
}

/// The outcome of a dry-run check — what an update pass *would* do. Nothing
/// here has been downloaded, verified, or installed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum UpdateDecision {
    /// The running version is at least as new as anything the channel offers.
    UpToDate { current: String },
    /// A newer release exists for this channel and platform.
    UpdateAvailable {
        current: String,
        version: String,
        url: String,
        signature: String,
        notes: String,
    },
    /// The channel has releases, but none ship an asset for this platform.
    UnsupportedPlatform { target: String },
    /// The channel has no releases at all.
    NoReleaseForChannel { channel: Channel },
}

impl UpdateDecision {
    /// True only when a newer, installable release was found.
    pub fn update_available(&self) -> bool {
        matches!(self, UpdateDecision::UpdateAvailable { .. })
    }
}

/// Whether a downloaded artifact's signature can actually be verified.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SigningStatus {
    /// No public key is configured, so no signature can be trusted yet.
    /// This is the current state until release keys exist (Phase 8).
    NotConfigured,
    /// A public key is configured (reserved for when signing lands).
    Configured,
}

/// The public key the updater would verify artifact signatures against.
/// Empty until release signing keys are generated in Phase 8, so
/// [`verification_status`] reports [`SigningStatus::NotConfigured`].
pub const UPDATE_PUBLIC_KEY: &str = "";

/// The channel endpoint template the app polls for a release manifest. The
/// `{channel}` placeholder is filled per check. The host is a placeholder
/// until real release infrastructure exists (Phase 8, Package & Release);
/// the `tauri-plugin-updater` wiring consumes this same endpoint + the
/// public key above once both are real.
pub const UPDATE_ENDPOINT_TEMPLATE: &str =
    "https://releases.llmneurosurgeon.dev/{channel}/latest.json";

/// The concrete manifest URL for a channel. Read-only config resolution; the
/// dry-run never actually fetches it in this build.
pub fn endpoint_for(channel: Channel) -> String {
    let name = match channel {
        Channel::Stable => "stable",
        Channel::Beta => "beta",
    };
    UPDATE_ENDPOINT_TEMPLATE.replace("{channel}", name)
}

/// Reports whether artifact signatures can be verified. A dry-run must never
/// claim an update is "verified" while this is `NotConfigured`.
pub fn verification_status() -> SigningStatus {
    if UPDATE_PUBLIC_KEY.is_empty() {
        SigningStatus::NotConfigured
    } else {
        SigningStatus::Configured
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateError {
    UnknownChannel(String),
    BadVersion(String),
    Manifest(String),
}

impl std::fmt::Display for UpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateError::UnknownChannel(c) => write!(f, "unknown update channel: {c}"),
            UpdateError::BadVersion(v) => write!(f, "not a valid semver version: {v}"),
            UpdateError::Manifest(m) => write!(f, "malformed release manifest: {m}"),
        }
    }
}

impl std::error::Error for UpdateError {}

/// The target-platform id for the current build, matching the keys used in a
/// [`Release`]'s `platforms` map.
pub fn current_target() -> &'static str {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        "linux-x86_64"
    }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        "linux-aarch64"
    }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        "darwin-x86_64"
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        "darwin-aarch64"
    }
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        "windows-x86_64"
    }
    #[cfg(not(any(
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "windows", target_arch = "x86_64"),
    )))]
    {
        "unsupported"
    }
}

/// The dry-run core: decides what an update pass would do, touching nothing.
///
/// `current` is the running version, `manifest` the channel document,
/// `channel` the channel to consider, and `target` the platform id to match
/// (`platforms` map key). Picks the newest channel-eligible release that
/// ships an asset for `target` and compares it to `current`.
pub fn check_for_update(
    current: &str,
    manifest: &ReleaseManifest,
    channel: Channel,
    target: &str,
) -> Result<UpdateDecision, UpdateError> {
    let current_version =
        Version::parse(current).map_err(|_| UpdateError::BadVersion(current.to_string()))?;

    if manifest.releases.is_empty() {
        return Ok(UpdateDecision::NoReleaseForChannel { channel });
    }

    // Parse + filter to channel-eligible releases. A release whose version
    // doesn't parse is skipped, not fatal — one malformed entry shouldn't
    // block updates for everyone.
    let mut eligible: Vec<(Version, &Release)> = manifest
        .releases
        .iter()
        .filter_map(|r| Version::parse(&r.version).ok().map(|v| (v, r)))
        .filter(|(v, _)| channel.accepts(v))
        .collect();

    if eligible.is_empty() {
        return Ok(UpdateDecision::NoReleaseForChannel { channel });
    }

    // Newest first.
    eligible.sort_by(|(a, _), (b, _)| b.cmp(a));

    // The newest release that actually ships an asset for this platform.
    let Some((version, release, asset)) = eligible
        .iter()
        .find_map(|(v, r)| r.platforms.get(target).map(|a| (v, *r, a)))
    else {
        return Ok(UpdateDecision::UnsupportedPlatform {
            target: target.to_string(),
        });
    };

    if *version > current_version {
        Ok(UpdateDecision::UpdateAvailable {
            current: current_version.to_string(),
            version: version.to_string(),
            url: asset.url.clone(),
            signature: asset.signature.clone(),
            notes: release.notes.clone(),
        })
    } else {
        Ok(UpdateDecision::UpToDate {
            current: current_version.to_string(),
        })
    }
}

/// A full dry-run result, ready to hand to a UI: the decision, a
/// human-facing report, and the signing status so the caller can never
/// present an update as installable while verification isn't wired up.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DryRunResult {
    pub decision: UpdateDecision,
    pub report: String,
    pub signing: SigningStatus,
}

/// One-call dry-run over a manifest JSON string, for the app's
/// `check_for_update` command. Parses the manifest, runs the channel check
/// for the current build's platform, and bundles the decision + report +
/// signing status. Downloads/installs nothing.
pub fn dry_run_from_json(
    current: &str,
    manifest_json: &str,
    channel: &str,
) -> Result<DryRunResult, UpdateError> {
    let channel = Channel::parse(channel)?;
    let manifest = ReleaseManifest::from_json(manifest_json)?;
    let decision = check_for_update(current, &manifest, channel, current_target())?;
    Ok(DryRunResult {
        report: dry_run_report(&decision),
        signing: verification_status(),
        decision,
    })
}

/// A human-facing, non-destructive summary of a dry-run decision. Always
/// states the signing status so an "update available" line is never
/// mistaken for "safe to install".
pub fn dry_run_report(decision: &UpdateDecision) -> String {
    match decision {
        UpdateDecision::UpToDate { current } => {
            format!("Up to date (running {current}). No update would be downloaded.")
        }
        UpdateDecision::UpdateAvailable {
            current,
            version,
            url,
            notes,
            ..
        } => {
            let signing = match verification_status() {
                SigningStatus::NotConfigured => {
                    "signature verification PENDING (release signing keys not configured — \
                     nothing would actually be installed in this build)"
                }
                SigningStatus::Configured => "signature would be verified before install",
            };
            let notes = if notes.is_empty() {
                String::new()
            } else {
                format!("\n  notes: {notes}")
            };
            format!("Update available: {current} → {version}\n  source: {url}\n  {signing}{notes}")
        }
        UpdateDecision::UnsupportedPlatform { target } => {
            format!("No update artifact is published for this platform ({target}).")
        }
        UpdateDecision::NoReleaseForChannel { channel } => {
            format!("The {channel:?} channel currently offers no releases.")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asset(url: &str) -> PlatformAsset {
        PlatformAsset {
            url: url.to_string(),
            signature: "c2lnbmF0dXJl".to_string(), // "signature" in base64
        }
    }

    fn release(version: &str, target: &str) -> Release {
        let mut platforms = HashMap::new();
        platforms.insert(target.to_string(), asset(&format!("https://dl/{version}")));
        Release {
            version: version.to_string(),
            notes: format!("release {version}"),
            pub_date: "2026-07-14".to_string(),
            platforms,
        }
    }

    fn manifest(channel: Channel, releases: Vec<Release>) -> ReleaseManifest {
        ReleaseManifest { channel, releases }
    }

    #[test]
    fn channel_parses_case_insensitively_and_rejects_typos() {
        assert_eq!(Channel::parse("Stable").unwrap(), Channel::Stable);
        assert_eq!(Channel::parse("  BETA ").unwrap(), Channel::Beta);
        assert!(matches!(
            Channel::parse("nightly"),
            Err(UpdateError::UnknownChannel(_))
        ));
    }

    #[test]
    fn detects_a_newer_stable_release() {
        let m = manifest(
            Channel::Stable,
            vec![
                release("0.1.0", "linux-x86_64"),
                release("0.2.0", "linux-x86_64"),
            ],
        );
        let d = check_for_update("0.1.0", &m, Channel::Stable, "linux-x86_64").unwrap();
        match &d {
            UpdateDecision::UpdateAvailable {
                version,
                url,
                signature,
                ..
            } => {
                assert_eq!(version, "0.2.0");
                assert_eq!(url, "https://dl/0.2.0");
                assert!(!signature.is_empty(), "signature must be carried through");
            }
            other => panic!("expected UpdateAvailable, got {other:?}"),
        }
        assert!(d.update_available());
    }

    #[test]
    fn reports_up_to_date_when_running_the_latest() {
        let m = manifest(Channel::Stable, vec![release("1.0.0", "linux-x86_64")]);
        let d = check_for_update("1.0.0", &m, Channel::Stable, "linux-x86_64").unwrap();
        assert_eq!(
            d,
            UpdateDecision::UpToDate {
                current: "1.0.0".into()
            }
        );
        assert!(!d.update_available());

        // A newer running build than the channel offers is still up-to-date.
        let d2 = check_for_update("1.1.0", &m, Channel::Stable, "linux-x86_64").unwrap();
        assert!(matches!(d2, UpdateDecision::UpToDate { .. }));
    }

    #[test]
    fn stable_channel_ignores_prereleases_but_beta_takes_them() {
        let m = manifest(
            Channel::Beta,
            vec![
                release("1.0.0", "linux-x86_64"),
                release("1.1.0-beta.1", "linux-x86_64"),
            ],
        );
        // Stable sees only 1.0.0 → running 1.0.0 is up to date.
        let stable = check_for_update("1.0.0", &m, Channel::Stable, "linux-x86_64").unwrap();
        assert!(matches!(stable, UpdateDecision::UpToDate { .. }));

        // Beta sees the pre-release and offers it.
        let beta = check_for_update("1.0.0", &m, Channel::Beta, "linux-x86_64").unwrap();
        match beta {
            UpdateDecision::UpdateAvailable { version, .. } => assert_eq!(version, "1.1.0-beta.1"),
            other => panic!("expected beta UpdateAvailable, got {other:?}"),
        }
    }

    #[test]
    fn picks_the_highest_version_regardless_of_manifest_order() {
        let m = manifest(
            Channel::Stable,
            vec![
                release("0.9.0", "linux-x86_64"),
                release("2.0.0", "linux-x86_64"),
                release("1.5.0", "linux-x86_64"),
            ],
        );
        let d = check_for_update("0.1.0", &m, Channel::Stable, "linux-x86_64").unwrap();
        match d {
            UpdateDecision::UpdateAvailable { version, .. } => assert_eq!(version, "2.0.0"),
            other => panic!("expected 2.0.0, got {other:?}"),
        }
    }

    #[test]
    fn unsupported_platform_when_no_asset_matches() {
        let m = manifest(Channel::Stable, vec![release("2.0.0", "darwin-aarch64")]);
        let d = check_for_update("1.0.0", &m, Channel::Stable, "linux-x86_64").unwrap();
        assert_eq!(
            d,
            UpdateDecision::UnsupportedPlatform {
                target: "linux-x86_64".into()
            }
        );
    }

    #[test]
    fn empty_or_channel_mismatched_manifest_yields_no_release() {
        let empty = manifest(Channel::Stable, vec![]);
        assert_eq!(
            check_for_update("1.0.0", &empty, Channel::Stable, "linux-x86_64").unwrap(),
            UpdateDecision::NoReleaseForChannel {
                channel: Channel::Stable
            }
        );

        // Only pre-releases, but asked on the Stable channel → nothing eligible.
        let only_beta = manifest(Channel::Beta, vec![release("1.1.0-beta.1", "linux-x86_64")]);
        assert_eq!(
            check_for_update("1.0.0", &only_beta, Channel::Stable, "linux-x86_64").unwrap(),
            UpdateDecision::NoReleaseForChannel {
                channel: Channel::Stable
            }
        );
    }

    #[test]
    fn a_single_malformed_version_entry_does_not_block_updates() {
        let mut bad = release("not-a-version", "linux-x86_64");
        bad.notes = "garbage".into();
        let m = manifest(Channel::Stable, vec![bad, release("2.0.0", "linux-x86_64")]);
        let d = check_for_update("1.0.0", &m, Channel::Stable, "linux-x86_64").unwrap();
        match d {
            UpdateDecision::UpdateAvailable { version, .. } => assert_eq!(version, "2.0.0"),
            other => panic!("expected 2.0.0 despite one bad entry, got {other:?}"),
        }
    }

    #[test]
    fn a_bad_current_version_is_a_loud_error() {
        let m = manifest(Channel::Stable, vec![release("2.0.0", "linux-x86_64")]);
        assert!(matches!(
            check_for_update("v-oops", &m, Channel::Stable, "linux-x86_64"),
            Err(UpdateError::BadVersion(_))
        ));
    }

    #[test]
    fn dry_run_never_claims_verified_while_signing_is_unconfigured() {
        // This build ships no public key, so the report must say so and must
        // NOT imply the update is safe to install.
        assert_eq!(verification_status(), SigningStatus::NotConfigured);
        let d = UpdateDecision::UpdateAvailable {
            current: "0.1.0".into(),
            version: "0.2.0".into(),
            url: "https://dl/0.2.0".into(),
            signature: "c2ln".into(),
            notes: String::new(),
        };
        let report = dry_run_report(&d);
        assert!(report.contains("0.1.0 → 0.2.0"));
        assert!(
            report.to_lowercase().contains("pending"),
            "report must flag that verification is pending signing keys: {report}"
        );
        assert!(!report.to_lowercase().contains("verified before install"));
    }

    #[test]
    fn manifest_round_trips_through_json() {
        let m = manifest(Channel::Stable, vec![release("2.0.0", "linux-x86_64")]);
        let json = serde_json::to_string(&m).unwrap();
        let back = ReleaseManifest::from_json(&json).unwrap();
        assert_eq!(m, back);
    }

    #[test]
    fn endpoint_resolves_per_channel() {
        assert_eq!(
            endpoint_for(Channel::Stable),
            "https://releases.llmneurosurgeon.dev/stable/latest.json"
        );
        assert_eq!(
            endpoint_for(Channel::Beta),
            "https://releases.llmneurosurgeon.dev/beta/latest.json"
        );
        // No unfilled placeholder leaks through.
        assert!(!endpoint_for(Channel::Stable).contains("{channel}"));
    }

    #[test]
    fn dry_run_from_json_bundles_decision_report_and_signing() {
        let m = manifest(Channel::Stable, vec![release("9.9.9", current_target())]);
        let json = serde_json::to_string(&m).unwrap();
        let result = dry_run_from_json("0.1.0", &json, "stable").unwrap();
        assert!(result.decision.update_available());
        assert_eq!(result.signing, SigningStatus::NotConfigured);
        assert!(result.report.contains("0.1.0 → 9.9.9"));
    }

    #[test]
    fn current_target_is_a_known_platform_id_on_this_build() {
        // On any of the three supported desktop targets this is a real id; the
        // test simply guards that the mapping compiles and isn't empty.
        assert!(!current_target().is_empty());
    }
}
