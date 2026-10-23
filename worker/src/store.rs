use crate::{
    time,
    types::{Coverage, Edition, Evidence, History, Run},
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};
use wasm_bindgen::JsValue;
use worker::{D1Database, D1PreparedStatement, Env, Error, Result};

pub fn integer(value: i64) -> JsValue {
    // D1 expects a JS Number. JsValue::from(i64) produces an unsupported BigInt.
    JsValue::from_f64(value as f64)
}

pub fn nullable<T: Into<JsValue>>(value: Option<T>) -> JsValue {
    value.map(Into::into).unwrap_or(JsValue::NULL)
}

pub fn hash(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

pub fn error(message: impl ToString) -> Error {
    Error::RustError(message.to_string())
}

pub struct Store {
    db: D1Database,
    bucket: worker::Bucket,
}

#[derive(Deserialize)]
struct Payload {
    payload: String,
}
#[derive(Deserialize)]
struct Metadata {
    metadata: String,
}
#[derive(Deserialize)]
struct Owner {
    owner: String,
}
#[derive(Serialize, Deserialize)]
struct Archive {
    text: String,
    raw: String,
}
#[derive(Deserialize)]
struct Observation {
    source_id: String,
    observed_at: i64,
    status: String,
    discovered: i64,
    detail: Option<String>,
}

impl Store {
    pub fn new(env: &Env) -> Result<Self> {
        Ok(Self {
            db: env.d1("DB")?,
            bucket: env.bucket("EVIDENCE")?,
        })
    }

    pub fn statement(&self, sql: &str, values: &[JsValue]) -> Result<D1PreparedStatement> {
        self.db.prepare(sql).bind(values)
    }

    fn observed(result: worker::D1Result) -> Result<worker::D1Result> {
        if !result.success() {
            return Err(error(
                result
                    .error()
                    .unwrap_or_else(|| "D1 operation failed".into()),
            ));
        }
        worker::console_log!("fid_usage d1 {:?}", result.meta()?);
        Ok(result)
    }

    pub async fn execute(&self, sql: &str, values: &[JsValue]) -> Result<()> {
        Self::observed(self.statement(sql, values)?.run().await?)?;
        Ok(())
    }

    pub async fn query<T: DeserializeOwned>(
        &self,
        sql: &str,
        values: &[JsValue],
    ) -> Result<Vec<T>> {
        Self::observed(self.statement(sql, values)?.all().await?)?.results::<T>()
    }

    pub async fn acquire(&self, name: &str, owner: &str) -> Result<bool> {
        let now = time::now();
        let rows: Vec<Owner> = self
            .query(
                include_str!("../../sql/acquire.sql"),
                &[name.into(), owner.into(), integer(now + 840), integer(now)],
            )
            .await?;
        Ok(rows.first().is_some_and(|row| row.owner == owner))
    }

    pub async fn observation(
        &self,
        source: &str,
        started: i64,
        status: &str,
        discovered: usize,
        detail: Option<String>,
    ) -> Result<()> {
        let id = hash(&format!("{source}:{started}"));
        self.execute("INSERT OR REPLACE INTO observations(id,source_id,observed_at,status,discovered,detail) VALUES(?,?,?,?,?,?)",
            &[id.into(),source.into(),integer(started),status.into(),(discovered as i32).into(),nullable(detail)]).await
    }

    pub async fn artifact(&self, run: &str, stage: &str, value: &serde_json::Value) -> Result<()> {
        let content = serde_json::to_string(value).map_err(error)?;
        let bytes = content.len();
        self.bucket
            .put(format!("runs/{run}/{stage}.json"), content)
            .execute()
            .await?;
        worker::console_log!("fid_usage r2_write bytes={bytes}");
        Ok(())
    }

    pub async fn evidence(&self, mut evidence: Evidence, raw: String) -> Result<bool> {
        #[derive(Deserialize)]
        struct Existing {
            id: String,
        }
        let existing: Vec<Existing> = self
            .query(
                "SELECT id FROM documents WHERE url=? AND content_hash=?",
                &[
                    evidence.url.clone().into(),
                    evidence.content_hash.clone().into(),
                ],
            )
            .await?;
        if existing.iter().any(|e| !e.id.is_empty()) {
            return Ok(false);
        }
        #[derive(Deserialize)]
        struct Seen {
            first_seen: i64,
        }
        let seen: Vec<Seen> = self
            .query(
                "SELECT MIN(first_seen) AS first_seen FROM documents WHERE url=? HAVING COUNT(*)>0",
                &[evidence.url.clone().into()],
            )
            .await?;
        if let Some(old) = seen.first() {
            evidence.first_seen = old.first_seen;
        }
        let key = format!("documents/{}.json", evidence.id);
        // Objects are immutable by content hash; D1 only references successfully stored objects.
        let content = serde_json::to_string(&Archive {
            text: evidence.text.clone(),
            raw,
        })
        .map_err(error)?;
        let bytes = content.len();
        self.bucket.put(&key, content).execute().await?;
        worker::console_log!("fid_usage r2_write bytes={bytes}");
        let mut metadata = evidence.clone();
        metadata.text.clear();
        self.execute("INSERT OR IGNORE INTO documents(id,url,source_id,first_seen,retrieved_at,published_at,updated_at,content_hash,object_key,metadata) VALUES(?,?,?,?,?,?,?,?,?,?)",
            &[evidence.id.into(),evidence.url.into(),evidence.source_id.into(),integer(evidence.first_seen),integer(evidence.retrieved_at),
              nullable(evidence.published.map(|d|d.value())),nullable(evidence.updated.map(|d|d.value())),evidence.content_hash.into(),key.into(),serde_json::to_string(&metadata).map_err(error)?.into()]).await?;
        Ok(true)
    }

    async fn with_text(&self, mut evidence: Evidence) -> Result<Evidence> {
        let object = self
            .bucket
            .get(format!("documents/{}.json", evidence.id))
            .execute()
            .await?
            .ok_or_else(|| error("Evidence object is missing"))?;
        let body = object
            .body()
            .ok_or_else(|| error("Evidence object has no body"))?;
        let content = body.text().await?;
        worker::console_log!("fid_usage r2_read bytes={}", content.len());
        let archive: Archive = serde_json::from_str(&content).map_err(error)?;
        evidence.text = archive.text;
        Ok(evidence)
    }

    pub async fn load_evidence(&self, id: &str) -> Result<Evidence> {
        let rows: Vec<Metadata> = self
            .query("SELECT metadata FROM documents WHERE id=?", &[id.into()])
            .await?;
        let row = rows
            .first()
            .ok_or_else(|| error("Evidence metadata is missing"))?;
        self.with_text(serde_json::from_str(&row.metadata).map_err(error)?)
            .await
    }

    pub async fn candidates(&self, previous: i64, cutoff: i64) -> Result<Vec<Evidence>> {
        let rows: Vec<Metadata> = self
            .query(
                include_str!("../../sql/candidates.sql"),
                &[
                    integer(cutoff),
                    integer(cutoff),
                    integer(previous - 30 * 86400),
                ],
            )
            .await?;
        let mut evidence = Vec::new();
        for row in rows {
            let metadata: Evidence = serde_json::from_str(&row.metadata).map_err(error)?;
            evidence.push(self.with_text(metadata).await?);
        }
        Ok(evidence)
    }

    pub async fn edition(
        &self,
        date: Option<&str>,
        revision: Option<u32>,
    ) -> Result<Option<Edition>> {
        let rows: Vec<Payload> = match (date,revision) {
            (Some(date),Some(revision)) => self.query("SELECT payload FROM revisions WHERE edition_date=? AND revision=?",&[date.into(),revision.into()]).await?,
            (Some(date),None) => self.query("SELECT r.payload FROM editions e JOIN revisions r ON r.edition_date=e.edition_date AND r.revision=e.current_revision WHERE e.edition_date=?",&[date.into()]).await?,
            (None,_) => self.query("SELECT r.payload FROM editions e JOIN revisions r ON r.edition_date=e.edition_date AND r.revision=e.current_revision ORDER BY e.edition_date DESC LIMIT 1",&[]).await?,
        };
        rows.first()
            .map(|p| serde_json::from_str(&p.payload).map_err(error))
            .transpose()
    }

    pub async fn history(&self) -> Result<Vec<History>> {
        self.query("SELECT event_id,last_edition,last_fingerprint,story FROM events ORDER BY last_edition DESC LIMIT 350",&[]).await
    }

    pub async fn history_before(&self, date: &str) -> Result<Vec<History>> {
        self.query("SELECT event_id,edition_date last_edition,fingerprint last_fingerprint,story FROM (SELECT *,ROW_NUMBER() OVER(PARTITION BY event_id ORDER BY edition_date DESC) position FROM event_history WHERE edition_date<?) WHERE position=1 LIMIT 350", &[date.into()]).await
    }

    pub async fn coverage(&self, previous: i64, cutoff: i64) -> Result<Vec<Coverage>> {
        let rows: Vec<Observation> = self.query("SELECT source_id,observed_at,status,discovered,detail FROM observations WHERE observed_at>=? AND observed_at<=? ORDER BY observed_at",&[integer(previous),integer(cutoff)]).await?;
        let mut coverage = Vec::new();
        let ids = crate::sources::SOURCES
            .iter()
            .map(|s| s.id)
            .chain(["hacker-news", "targeted-search"]);
        for id in ids {
            let checks: Vec<_> = rows.iter().filter(|r| r.source_id == id).collect();
            let mut gaps = Vec::new();
            if checks.is_empty() {
                gaps.push("No checks observed in this collection window".into());
            }
            for row in &checks {
                if row.status != "ok" {
                    gaps.push(row.detail.clone().unwrap_or_else(|| row.status.clone()));
                }
            }
            let tolerance = if id == "hacker-news" {
                30 * 60
            } else {
                90 * 60
            };
            if id != "targeted-search" {
                let mut last = previous;
                for timestamp in checks
                    .iter()
                    .map(|r| r.observed_at)
                    .chain(std::iter::once(cutoff))
                {
                    if timestamp - last > tolerance {
                        gaps.push(format!(
                            "Observation gap: {} to {} UTC",
                            time::at(last)?.to_rfc3339(),
                            time::at(timestamp)?.to_rfc3339()
                        ));
                    }
                    last = timestamp;
                }
            }
            coverage.push(Coverage {
                source_id: id.into(),
                checks: checks.len(),
                successful_checks: checks.iter().filter(|r| r.status == "ok").count(),
                discovered: checks.iter().map(|r| r.discovered).sum(),
                gaps,
            });
        }
        Ok(coverage)
    }

    pub async fn latest_run(&self, date: &str) -> Result<Option<Run>> {
        Ok(self
            .query(
                "SELECT * FROM runs WHERE edition_date=? ORDER BY started_at DESC LIMIT 1",
                &[date.into()],
            )
            .await?
            .into_iter()
            .next())
    }

    pub async fn start_run(&self, id: &str, date: &str, cutoff: i64, started: i64) -> Result<()> {
        self.execute(
            "INSERT INTO runs(id,edition_date,cutoff,started_at,status) VALUES(?,?,?,?,'running')",
            &[id.into(), date.into(), integer(cutoff), integer(started)],
        )
        .await
    }

    pub async fn fail_run(&self, id: &str, detail: &str) -> Result<()> {
        self.execute("UPDATE runs SET status='incomplete',finished_at=?,detail=? WHERE id=? AND status='running'",&[integer(time::now()),detail.into(),id.into()]).await
    }

    pub async fn publish(
        &self,
        edition: &Edition,
        fingerprints: &[(String, String)],
        run: &str,
    ) -> Result<()> {
        let mut statements = vec![
            self.statement(
                include_str!("../../sql/publish.sql"),
                &[
                    edition.date.clone().into(),
                    integer(edition.cutoff),
                    integer(edition.generation_started_at),
                    run.into(),
                    integer(edition.cutoff + 3600),
                ],
            )?,
            self.statement(
                include_str!("../../sql/publish_revision.sql"),
                &[
                    edition.date.clone().into(),
                    serde_json::to_string(edition).map_err(error)?.into(),
                ],
            )?,
            self.statement(
                "UPDATE runs SET status='published',finished_at=(SELECT published_at FROM editions WHERE edition_date=?) WHERE id=?",
                &[edition.date.clone().into(), run.into()],
            )?,
        ];
        for (event, fingerprint) in fingerprints {
            let story = edition
                .stories
                .iter()
                .find(|s| s.event_id == *event)
                .ok_or_else(|| error("Missing event"))?;
            statements.push(self.statement("INSERT INTO events(event_id,last_edition,last_fingerprint,story) VALUES(?,?,?,?) ON CONFLICT(event_id) DO UPDATE SET last_edition=excluded.last_edition,last_fingerprint=excluded.last_fingerprint,story=excluded.story WHERE excluded.last_edition>=events.last_edition",&[event.clone().into(),edition.date.clone().into(),fingerprint.clone().into(),serde_json::to_string(story).map_err(error)?.into()])?);
        }
        for (event, fingerprint) in fingerprints {
            let story = edition
                .stories
                .iter()
                .find(|s| s.event_id == *event)
                .ok_or_else(|| error("Missing event"))?;
            statements.push(self.statement("INSERT INTO event_history(event_id,edition_date,fingerprint,story) VALUES(?,?,?,?)", &[event.clone().into(),edition.date.clone().into(),fingerprint.clone().into(),serde_json::to_string(story).map_err(error)?.into()])?);
        }
        for evidence in &edition.evidence {
            statements.push(self.statement(
                "INSERT OR IGNORE INTO published_evidence(id,edition_date) VALUES(?,?)",
                &[evidence.id.clone().into(), edition.date.clone().into()],
            )?);
        }
        for result in self.db.batch(statements).await? {
            Self::observed(result)?;
        }
        Ok(())
    }

    pub async fn correct(
        &self,
        edition: &Edition,
        previous: u32,
        reason: &str,
        fingerprints: &[(String, String)],
    ) -> Result<()> {
        let mut statements = vec![
            self.statement(
                include_str!("../../sql/correct_revision.sql"),
                &[
                    edition.date.clone().into(),
                    edition.revision.into(),
                    integer(time::now()),
                    reason.into(),
                    serde_json::to_string(edition).map_err(error)?.into(),
                    edition.date.clone().into(),
                    previous.into(),
                ],
            )?,
            self.statement(
                include_str!("../../sql/correct_pointer.sql"),
                &[
                    edition.revision.into(),
                    edition.date.clone().into(),
                    previous.into(),
                    edition.date.clone().into(),
                    edition.revision.into(),
                ],
            )?,
        ];
        let payload = serde_json::to_string(edition).map_err(error)?;
        for (event, fingerprint) in fingerprints {
            let story = edition
                .stories
                .iter()
                .find(|s| s.event_id == *event)
                .ok_or_else(|| error("Missing corrected event"))?;
            let text = serde_json::to_string(story).map_err(error)?;
            statements.push(self.statement("UPDATE events SET story=?,last_fingerprint=? WHERE event_id=? AND last_edition=? AND EXISTS(SELECT 1 FROM revisions WHERE edition_date=? AND revision=? AND payload=?)", &[text.clone().into(),fingerprint.clone().into(),event.clone().into(),edition.date.clone().into(),edition.date.clone().into(),edition.revision.into(),payload.clone().into()])?);
            statements.push(self.statement("UPDATE event_history SET story=?,fingerprint=? WHERE event_id=? AND edition_date=? AND EXISTS(SELECT 1 FROM revisions WHERE edition_date=? AND revision=? AND payload=?)", &[text.into(),fingerprint.clone().into(),event.clone().into(),edition.date.clone().into(),edition.date.clone().into(),edition.revision.into(),payload.clone().into()])?);
        }
        for result in self.db.batch(statements).await? {
            Self::observed(result)?;
        }
        Ok(())
    }
}
