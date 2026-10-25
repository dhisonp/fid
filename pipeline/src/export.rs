use crate::{sources, store::Store, time, types::PublicEdition, Result};
use serde::Serialize;
use serde_json::json;
use std::{collections::BTreeMap, fs, path::Path};

fn write(path: &Path, value: &impl Serialize) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec(value)?)?;
    Ok(())
}

pub fn site(store: &Store, output: &Path) -> Result<()> {
    let editions = store.editions()?;
    let expected = time::daily(time::now())?;
    let feed = |edition: Option<&crate::types::Edition>| {
        json!({
            "status": match edition {
                None => "unavailable",
                Some(e) if e.date < expected.date => "stale",
                Some(_) => "available",
            },
            "expected_date": expected.date,
            "checked_at": time::now(),
            "edition": edition.map(PublicEdition::from),
            "run": null,
        })
    };
    let mut current = BTreeMap::new();
    for edition in &editions {
        write(
            &output.join(format!(
                "data/editions/{}/revisions/{}.json",
                edition.date, edition.revision
            )),
            &feed(Some(edition)),
        )?;
        for evidence in &edition.evidence {
            write(
                &output.join(format!("data/evidence/{}.json", evidence.id)),
                evidence,
            )?;
        }
        current.insert(edition.date.as_str(), edition);
    }
    for edition in current.values() {
        write(
            &output.join(format!("data/editions/{}.json", edition.date)),
            &feed(Some(edition)),
        )?;
    }
    write(&output.join("data/latest.json"), &feed(editions.last()))?;
    let summaries: Vec<_> = current.values().rev().take(90).map(|edition| {
        json!({"edition_date": edition.date, "published_at": edition.published_at, "current_revision": edition.revision})
    }).collect();
    write(&output.join("data/editions.json"), &summaries)?;
    write(&output.join("data/sources.json"), &sources::SOURCES)?;
    Ok(())
}
