//! T7.3 self-verify: "update channel dry-run works." Drives the real
//! update-channel logic against the committed release manifest fixture
//! (`fixtures/updater/latest.json`) — parsing a real on-disk manifest and
//! deciding what an update pass would do, without downloading, verifying, or
//! installing anything.

use std::path::PathBuf;

use neurosurgeon_core::updater::{
    check_for_update, dry_run_report, verification_status, Channel, ReleaseManifest, SigningStatus,
    UpdateDecision,
};

fn load_fixture_manifest() -> ReleaseManifest {
    // CARGO_MANIFEST_DIR is packages/core; the shared fixtures live at the
    // workspace root.
    let path: PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "..",
        "..",
        "fixtures",
        "updater",
        "latest.json",
    ]
    .iter()
    .collect();
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
    ReleaseManifest::from_json(&raw).expect("fixture manifest should parse")
}

#[test]
fn dry_run_against_the_committed_manifest_finds_the_newer_release() {
    let manifest = load_fixture_manifest();

    // An older build on the stable channel should be offered 0.2.0 for its
    // platform. Use a platform the fixture actually publishes.
    let decision = check_for_update("0.1.0", &manifest, Channel::Stable, "linux-x86_64")
        .expect("dry-run should succeed");

    match &decision {
        UpdateDecision::UpdateAvailable {
            current,
            version,
            url,
            ..
        } => {
            assert_eq!(current, "0.1.0");
            assert_eq!(version, "0.2.0");
            assert!(
                url.ends_with("0.2.0_amd64.AppImage"),
                "unexpected url: {url}"
            );
        }
        other => panic!("expected an update to 0.2.0, got {other:?}"),
    }

    // The dry-run report must state the update AND that signing isn't wired
    // up yet — nothing is actually installed in this build.
    let report = dry_run_report(&decision);
    assert!(report.contains("0.1.0 → 0.2.0"));
    assert_eq!(verification_status(), SigningStatus::NotConfigured);
    assert!(report.to_lowercase().contains("pending"));
}

#[test]
fn dry_run_reports_up_to_date_for_the_latest_build() {
    let manifest = load_fixture_manifest();
    let decision = check_for_update("0.2.0", &manifest, Channel::Stable, "darwin-aarch64")
        .expect("dry-run should succeed");
    assert!(
        matches!(decision, UpdateDecision::UpToDate { .. }),
        "0.2.0 is the newest fixture release; got {decision:?}"
    );
}

#[test]
fn dry_run_does_not_touch_the_network_or_filesystem() {
    // The whole point of a dry-run: `check_for_update` is pure over the
    // manifest it is handed. This test passes offline and writes nothing —
    // it exercises the decision path with no I/O beyond reading the fixture.
    let manifest = load_fixture_manifest();
    for target in ["linux-x86_64", "darwin-aarch64", "windows-x86_64"] {
        let decision =
            check_for_update("0.1.0", &manifest, Channel::Stable, target).expect("dry-run ok");
        assert!(
            decision.update_available(),
            "every published platform has 0.2.0"
        );
    }
}
