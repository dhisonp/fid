use fid::{
    export,
    store::{hash, Store},
    time,
    types::{Edition, Evidence},
};
use serde_json::{json, Value};
use std::{fs, process::Command};

fn edition() -> (Edition, String, String) {
    let raw = "<article>An originating document with an important technical change.</article>";
    let url = "https://example.com/release";
    let id = hash(&format!("{url}:{raw}"));
    let window = time::daily(time::now()).unwrap();
    let evidence = Evidence {
        id: id.clone(),
        url: url.into(),
        source_id: "feed".into(),
        title: "Release".into(),
        published: Some(fid::types::SourceDate::Day(window.date.parse().unwrap())),
        updated: None,
        first_seen: window.cutoff - 1,
        retrieved_at: window.cutoff,
        content_hash: hash(raw),
        text: "An originating document with an important technical change.".into(),
    };
    let fingerprint = hash(&format!("{}:{}", evidence.url, evidence.content_hash));
    let edition = Edition {
        verification: false, date: window.date, revision: 1, cutoff: window.cutoff,
        generation_started_at: window.cutoff, published_at: window.cutoff,
        stories: vec![serde_json::from_value(json!({
            "event_id":"release", "category":"developer-tools", "headline":"A release",
            "paragraphs":["An important technical change."], "claims":[], "evidence_ids":[id],
            "label":"new", "delta":"", "uncertainty":"", "score":90, "rationale":"Useful", "paper":null,
        })).unwrap()],
        evidence: vec![evidence], coverage: vec![], corrections: vec![],
    };
    (edition, raw.into(), fingerprint)
}

fn publish(store: &Store, edition: &Edition, raw: String, fingerprint: String) {
    store.evidence(edition.evidence[0].clone(), raw).unwrap();
    assert!(store.acquire("generation", "run").unwrap());
    store
        .start_run("run", &edition.date, edition.cutoff, edition.cutoff)
        .unwrap();
    store
        .publish(edition, &[("release".into(), fingerprint)], "run")
        .unwrap();
    store.snapshot().unwrap();
}

#[test]
fn saved_publication_survives_a_fresh_runner_and_exports_only_reader_fields() {
    let state = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();
    let store = Store::open(state.path()).unwrap();
    let (edition, raw, fingerprint) = edition();
    publish(&store, &edition, raw, fingerprint.clone());
    drop(store);

    let restored = Store::open(state.path()).unwrap();
    restored.restore().unwrap();
    assert_eq!(restored.history().unwrap()[0].last_fingerprint, fingerprint);
    let saved = restored.edition(None, None).unwrap().unwrap();
    assert_eq!(saved.evidence[0].published, edition.evidence[0].published);
    assert_eq!(saved.evidence[0].text, edition.evidence[0].text);
    export::site(&restored, output.path()).unwrap();
    let latest: Value =
        serde_json::from_slice(&fs::read(output.path().join("data/latest.json")).unwrap()).unwrap();
    assert_eq!(latest["edition"]["date"], edition.date);
    for field in ["category", "claims", "score", "rationale", "paper"] {
        assert!(latest["edition"]["stories"][0].get(field).is_none());
    }
    assert!(latest["edition"]["evidence"][0].get("text").is_none());
    assert!(output
        .path()
        .join(format!("data/editions/{}/revisions/1.json", edition.date))
        .is_file());
    assert!(!output.path().join("work").exists());

    let rerun = Command::new(env!("CARGO_BIN_EXE_fid"))
        .args(["daily", "--state"])
        .arg(state.path())
        .env_remove("GROK_API_KEY")
        .output()
        .unwrap();
    assert!(
        rerun.status.success(),
        "{}",
        String::from_utf8_lossy(&rerun.stderr)
    );
    assert!(String::from_utf8_lossy(&rerun.stdout).contains("already published"));
}

#[test]
fn corrupted_archives_and_missing_manifests_fail_instead_of_resetting_history() {
    let state = tempfile::tempdir().unwrap();
    let store = Store::open(state.path()).unwrap();
    let (edition, raw, fingerprint) = edition();
    publish(&store, &edition, raw, fingerprint);
    fs::write(
        state
            .path()
            .join(format!("documents/{}.json", edition.evidence[0].id)),
        r#"{"text":"corrupted","raw":"corrupted"}"#,
    )
    .unwrap();
    // Simulate a fresh runner, which has no temporary collection archive.
    fs::remove_dir_all(state.path().join("work")).unwrap();
    let restored = Store::open(state.path()).unwrap();
    assert!(restored.restore().is_err());
    fs::remove_file(state.path().join("state.json")).unwrap();
    assert!(Store::open(state.path()).is_err());
}

#[test]
fn lost_leases_and_duplicate_publications_cannot_replace_an_edition() {
    let state = tempfile::tempdir().unwrap();
    let store = Store::open(state.path()).unwrap();
    let (edition, raw, fingerprint) = edition();
    publish(&store, &edition, raw, fingerprint.clone());
    assert!(store
        .publish(&edition, &[("release".into(), fingerprint)], "run")
        .is_err());
    assert_eq!(store.editions().unwrap().len(), 1);
    let mut next = edition.clone();
    next.date = "2099-01-01".into();
    assert!(store
        .publish(&next, &[("release".into(), "hash".into())], "wrong-owner")
        .is_err());
    assert_eq!(store.editions().unwrap().len(), 1);
}

#[test]
fn a_first_deployment_without_an_edition_has_an_honest_empty_state() {
    let state = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();
    let store = Store::open(state.path()).unwrap();
    export::site(&store, output.path()).unwrap();
    let latest: Value =
        serde_json::from_slice(&fs::read(output.path().join("data/latest.json")).unwrap()).unwrap();
    assert_eq!(latest["status"], "unavailable");
    assert_eq!(latest["edition"], Value::Null);
}

#[test]
fn restoration_keeps_original_revisions_and_the_latest_corrected_history() {
    let state = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();
    let store = Store::open(state.path()).unwrap();
    let (mut edition, raw, fingerprint) = edition();
    publish(&store, &edition, raw, fingerprint.clone());
    for revision in 2..=12 {
        edition.revision = revision;
        edition.stories[0].headline = format!("Corrected release {revision}");
        store
            .correct(
                &edition,
                revision - 1,
                "Correct the affected version",
                &[("release".into(), fingerprint.clone())],
            )
            .unwrap();
    }
    store.snapshot().unwrap();
    let restored = Store::open(state.path()).unwrap();
    restored.restore().unwrap();
    assert_eq!(restored.edition(None, None).unwrap().unwrap().revision, 12);
    assert_eq!(
        restored
            .edition(Some(&edition.date), Some(1))
            .unwrap()
            .unwrap()
            .stories[0]
            .headline,
        "A release"
    );
    let history: Value = serde_json::from_str(&restored.history().unwrap()[0].story).unwrap();
    assert_eq!(history["headline"], "Corrected release 12");
    export::site(&restored, output.path()).unwrap();
    let original: Value = serde_json::from_slice(
        &fs::read(
            output
                .path()
                .join(format!("data/editions/{}/revisions/1.json", edition.date)),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(original["edition"]["stories"][0]["headline"], "A release");
}
