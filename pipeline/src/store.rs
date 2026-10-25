use crate::{
    time,
    types::{Coverage, Edition, Evidence, History, Run},
    Error, Result,
};
use rusqlite::{
    params_from_iter,
    types::{Value, ValueRef},
    Connection,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    cell::RefCell,
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone)]
pub struct SqlValue(Value);
impl From<String> for SqlValue {
    fn from(value: String) -> Self {
        Self(Value::Text(value))
    }
}
impl From<&str> for SqlValue {
    fn from(value: &str) -> Self {
        value.to_string().into()
    }
}
impl From<i32> for SqlValue {
    fn from(value: i32) -> Self {
        integer(i64::from(value))
    }
}
impl From<u32> for SqlValue {
    fn from(value: u32) -> Self {
        integer(i64::from(value))
    }
}
impl From<f64> for SqlValue {
    fn from(value: f64) -> Self {
        Self(Value::Real(value))
    }
}

pub fn integer(value: i64) -> SqlValue {
    SqlValue(Value::Integer(value))
}
pub fn nullable<T: Into<SqlValue>>(value: Option<T>) -> SqlValue {
    value.map(Into::into).unwrap_or(SqlValue(Value::Null))
}
pub fn hash(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}
pub fn error(message: impl ToString) -> Error {
    Error::Invalid(message.to_string())
}

fn validate_id(id: &str) -> Result<()> {
    if id.len() != 64 || !id.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err(error("Invalid evidence identity"));
    }
    Ok(())
}

pub struct Store {
    db: RefCell<Connection>,
    root: PathBuf,
}

struct Statement {
    sql: String,
    values: Vec<SqlValue>,
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
    status: String,
    discovered: i64,
    detail: Option<String>,
}

impl Store {
    pub fn open(root: &Path) -> Result<Self> {
        #[derive(Serialize, Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Manifest {
            version: u32,
        }
        let manifest = root.join("state.json");
        if manifest.exists() {
            let manifest: Manifest = serde_json::from_slice(&fs::read(&manifest)?)?;
            if manifest.version != 1 {
                return Err(error("Unsupported publication state version"));
            }
        } else if root.join("editions").exists() || root.join("documents").exists() {
            return Err(error("Publication history is missing its state manifest"));
        }
        fs::create_dir_all(root.join("work/documents"))?;
        if !manifest.exists() {
            fs::write(manifest, serde_json::to_vec(&Manifest { version: 1 })?)?;
        }
        let db = Connection::open_in_memory()?;
        db.execute_batch(include_str!("../../migrations/0001_initial.sql"))?;
        Ok(Self {
            db: RefCell::new(db),
            root: root.to_owned(),
        })
    }

    fn statement(sql: &str, values: &[SqlValue]) -> Statement {
        Statement {
            sql: sql.to_owned(),
            values: values.to_vec(),
        }
    }

    fn batch(&self, statements: Vec<Statement>) -> Result<()> {
        let mut db = self.db.borrow_mut();
        let transaction = db.transaction()?;
        for statement in statements {
            transaction.execute(
                &statement.sql,
                params_from_iter(statement.values.iter().map(|v| &v.0)),
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn execute(&self, sql: &str, values: &[SqlValue]) -> Result<()> {
        self.db
            .borrow()
            .execute(sql, params_from_iter(values.iter().map(|v| &v.0)))?;
        Ok(())
    }

    pub fn query<T: DeserializeOwned>(&self, sql: &str, values: &[SqlValue]) -> Result<Vec<T>> {
        let db = self.db.borrow();
        let mut statement = db.prepare(sql)?;
        let names: Vec<String> = statement
            .column_names()
            .into_iter()
            .map(str::to_owned)
            .collect();
        let mut rows = statement.query(params_from_iter(values.iter().map(|v| &v.0)))?;
        let mut result = Vec::new();
        while let Some(row) = rows.next()? {
            let mut object = serde_json::Map::new();
            for (index, name) in names.iter().enumerate() {
                let value = match row.get_ref(index)? {
                    ValueRef::Null => serde_json::Value::Null,
                    ValueRef::Integer(v) => v.into(),
                    ValueRef::Real(v) => serde_json::Number::from_f64(v)
                        .ok_or_else(|| error("Nonfinite database number"))?
                        .into(),
                    ValueRef::Text(v) => std::str::from_utf8(v).map_err(error)?.into(),
                    ValueRef::Blob(_) => return Err(error("Unexpected database blob")),
                };
                object.insert(name.clone(), value);
            }
            result.push(serde_json::from_value(object.into())?);
        }
        Ok(result)
    }

    fn archive_path(&self, id: &str) -> PathBuf {
        let collected = self.root.join("work/documents").join(format!("{id}.json"));
        if collected.exists() {
            collected
        } else {
            self.root.join("documents").join(format!("{id}.json"))
        }
    }

    pub fn restore(&self) -> Result<()> {
        let directory = self.root.join("editions");
        if !directory.exists() {
            return Ok(());
        }
        let mut files = Vec::new();
        for date in fs::read_dir(directory)? {
            let date = date?;
            if !date.file_type()?.is_dir() {
                return Err(error("Unexpected file in edition history"));
            }
            let date_name = date
                .file_name()
                .into_string()
                .map_err(|_| error("Invalid edition filename"))?;
            let parsed =
                chrono::NaiveDate::parse_from_str(&date_name, "%Y-%m-%d").map_err(error)?;
            if parsed.to_string() != date_name {
                return Err(error("Invalid edition date"));
            }
            for revision in fs::read_dir(date.path())? {
                let revision = revision?;
                let path = revision.path();
                if !revision.file_type()?.is_file()
                    || path.extension().and_then(|s| s.to_str()) != Some("json")
                {
                    return Err(error("Unexpected revision file"));
                }
                let number = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .ok_or_else(|| error("Invalid revision filename"))?
                    .parse::<u32>()
                    .map_err(error)?;
                if number == 0 {
                    return Err(error("Revision must be positive"));
                }
                files.push((date_name.clone(), number, path));
            }
        }
        files.sort_by(|a, b| (&a.0, a.1).cmp(&(&b.0, b.1)));
        for (date, revision, file) in files {
            let edition: Edition = serde_json::from_slice(&fs::read(file)?)?;
            if edition.date != date || edition.revision != revision {
                return Err(error("Stored edition identity does not match its filename"));
            }
            self.import(&edition)?;
        }
        Ok(())
    }

    fn import(&self, edition: &Edition) -> Result<()> {
        let mut statements = Vec::new();
        for evidence in &edition.evidence {
            validate_id(&evidence.id)?;
            let archive: Archive =
                serde_json::from_slice(&fs::read(self.archive_path(&evidence.id))?)?;
            if evidence.id != hash(&format!("{}:{}", evidence.url, archive.raw))
                || evidence.text != archive.text
            {
                return Err(error("Stored evidence does not match its archive"));
            }
            let mut metadata = evidence.clone();
            metadata.text.clear();
            statements.push(Self::statement("INSERT OR IGNORE INTO documents(id,url,source_id,first_seen,retrieved_at,content_hash,object_key,metadata) VALUES(?,?,?,?,?,?,?,?)",
                &[evidence.id.clone().into(), evidence.url.clone().into(), evidence.source_id.clone().into(), integer(evidence.first_seen), integer(evidence.retrieved_at), evidence.content_hash.clone().into(), format!("documents/{}.json",evidence.id).into(),serde_json::to_string(&metadata)?.into()]));
        }
        statements.push(Self::statement("INSERT INTO editions(edition_date,cutoff,generation_started_at,published_at,current_revision) VALUES(?,?,?,?,?) ON CONFLICT(edition_date) DO UPDATE SET current_revision=MAX(current_revision,excluded.current_revision)",
            &[edition.date.clone().into(), integer(edition.cutoff), integer(edition.generation_started_at), integer(edition.published_at), edition.revision.into()]));
        statements.push(Self::statement("INSERT OR IGNORE INTO revisions(edition_date,revision,created_at,payload) VALUES(?,?,?,?)", &[edition.date.clone().into(),edition.revision.into(),integer(edition.published_at),serde_json::to_string(edition)?.into()]));
        for story in &edition.stories {
            let fingerprint = crate::editorial::fingerprint(story, &edition.evidence)?;
            let text = serde_json::to_string(story)?;
            statements.push(Self::statement("INSERT INTO events(event_id,last_edition,last_fingerprint,story) VALUES(?,?,?,?) ON CONFLICT(event_id) DO UPDATE SET last_edition=excluded.last_edition,last_fingerprint=excluded.last_fingerprint,story=excluded.story WHERE excluded.last_edition>=events.last_edition", &[story.event_id.clone().into(),edition.date.clone().into(),fingerprint.clone().into(),text.clone().into()]));
            statements.push(Self::statement("INSERT OR REPLACE INTO event_history(event_id,edition_date,fingerprint,story) VALUES(?,?,?,?)", &[story.event_id.clone().into(),edition.date.clone().into(),fingerprint.into(),text.into()]));
        }
        for evidence in &edition.evidence {
            statements.push(Self::statement(
                "INSERT OR IGNORE INTO published_evidence(id,edition_date) VALUES(?,?)",
                &[evidence.id.clone().into(), edition.date.clone().into()],
            ));
        }
        self.batch(statements)
    }

    pub fn editions(&self) -> Result<Vec<Edition>> {
        let rows: Vec<Payload> = self.query(
            "SELECT payload FROM revisions ORDER BY edition_date,revision",
            &[],
        )?;
        rows.into_iter()
            .map(|p| serde_json::from_str(&p.payload).map_err(Into::into))
            .collect()
    }

    pub fn snapshot(&self) -> Result<()> {
        fs::create_dir_all(self.root.join("documents"))?;
        for edition in self.editions()? {
            for evidence in &edition.evidence {
                let destination = self
                    .root
                    .join("documents")
                    .join(format!("{}.json", evidence.id));
                if !destination.exists() {
                    let temporary = destination.with_extension("tmp");
                    fs::copy(self.archive_path(&evidence.id), &temporary)?;
                    fs::rename(temporary, destination)?;
                }
            }
            let directory = self.root.join("editions").join(&edition.date);
            fs::create_dir_all(&directory)?;
            let destination = directory.join(format!("{}.json", edition.revision));
            if !destination.exists() {
                let temporary = destination.with_extension("tmp");
                fs::write(&temporary, serde_json::to_vec(&edition)?)?;
                fs::rename(temporary, destination)?;
            }
        }
        Ok(())
    }

    pub fn acquire(&self, name: &str, owner: &str) -> Result<bool> {
        let now = time::now();
        let rows: Vec<Owner> = self.query(
            include_str!("../../sql/acquire.sql"),
            &[name.into(), owner.into(), integer(now + 840), integer(now)],
        )?;
        Ok(rows.first().is_some_and(|row| row.owner == owner))
    }

    pub fn observation(
        &self,
        source: &str,
        started: i64,
        status: &str,
        discovered: usize,
        detail: Option<String>,
    ) -> Result<()> {
        let id = hash(&format!("{source}:{started}"));
        self.execute("INSERT OR REPLACE INTO observations(id,source_id,observed_at,status,discovered,detail) VALUES(?,?,?,?,?,?)",
            &[id.into(),source.into(),integer(started),status.into(),(discovered as i32).into(),nullable(detail)])
    }

    pub fn artifact(&self, run: &str, stage: &str, value: &serde_json::Value) -> Result<()> {
        let content = serde_json::to_string(value).map_err(error)?;
        let directory = self.root.join("work/runs").join(run);
        fs::create_dir_all(&directory)?;
        fs::write(directory.join(format!("{stage}.json")), content)?;
        Ok(())
    }

    pub fn evidence(&self, mut evidence: Evidence, raw: String) -> Result<bool> {
        #[derive(Deserialize)]
        struct Existing {
            id: String,
        }
        let existing: Vec<Existing> = self.query(
            "SELECT id FROM documents WHERE url=? AND content_hash=?",
            &[
                evidence.url.clone().into(),
                evidence.content_hash.clone().into(),
            ],
        )?;
        if existing.iter().any(|e| !e.id.is_empty()) {
            return Ok(false);
        }
        #[derive(Deserialize)]
        struct Seen {
            first_seen: i64,
        }
        let seen: Vec<Seen> = self.query(
            "SELECT MIN(first_seen) AS first_seen FROM documents WHERE url=? HAVING COUNT(*)>0",
            &[evidence.url.clone().into()],
        )?;
        if let Some(old) = seen.first() {
            evidence.first_seen = old.first_seen;
        }
        let key = format!("documents/{}.json", evidence.id);
        // Objects are immutable by content hash; SQLite only references successfully stored objects.
        let content = serde_json::to_string(&Archive {
            text: evidence.text.clone(),
            raw,
        })
        .map_err(error)?;
        fs::write(self.root.join("work").join(&key), content)?;
        let mut metadata = evidence.clone();
        metadata.text.clear();
        self.execute("INSERT OR IGNORE INTO documents(id,url,source_id,first_seen,retrieved_at,published_at,updated_at,content_hash,object_key,metadata) VALUES(?,?,?,?,?,?,?,?,?,?)",
            &[evidence.id.into(),evidence.url.into(),evidence.source_id.into(),integer(evidence.first_seen),integer(evidence.retrieved_at),
              nullable(evidence.published.map(|d|d.value())),nullable(evidence.updated.map(|d|d.value())),evidence.content_hash.into(),key.into(),serde_json::to_string(&metadata).map_err(error)?.into()])?;
        Ok(true)
    }

    fn with_text(&self, mut evidence: Evidence) -> Result<Evidence> {
        let archive: Archive = serde_json::from_slice(&fs::read(self.archive_path(&evidence.id))?)?;
        evidence.text = archive.text;
        Ok(evidence)
    }

    pub fn load_evidence(&self, id: &str) -> Result<Evidence> {
        let rows: Vec<Metadata> =
            self.query("SELECT metadata FROM documents WHERE id=?", &[id.into()])?;
        let row = rows
            .first()
            .ok_or_else(|| error("Evidence metadata is missing"))?;
        self.with_text(serde_json::from_str(&row.metadata).map_err(error)?)
    }

    pub fn candidates(&self, previous: i64, cutoff: i64) -> Result<Vec<Evidence>> {
        let rows: Vec<Metadata> = self.query(
            include_str!("../../sql/candidates.sql"),
            &[
                integer(cutoff),
                integer(cutoff),
                integer(previous - 30 * 86400),
            ],
        )?;
        let mut evidence = Vec::new();
        for row in rows {
            let metadata: Evidence = serde_json::from_str(&row.metadata).map_err(error)?;
            evidence.push(self.with_text(metadata)?);
        }
        Ok(evidence)
    }

    pub fn edition(&self, date: Option<&str>, revision: Option<u32>) -> Result<Option<Edition>> {
        let rows: Vec<Payload> = match (date,revision) {
            (Some(date),Some(revision)) => self.query("SELECT payload FROM revisions WHERE edition_date=? AND revision=?",&[date.into(),revision.into()])?,
            (Some(date),None) => self.query("SELECT r.payload FROM editions e JOIN revisions r ON r.edition_date=e.edition_date AND r.revision=e.current_revision WHERE e.edition_date=?",&[date.into()])?,
            (None,_) => self.query("SELECT r.payload FROM editions e JOIN revisions r ON r.edition_date=e.edition_date AND r.revision=e.current_revision ORDER BY e.edition_date DESC LIMIT 1",&[])?,
        };
        rows.first()
            .map(|p| serde_json::from_str(&p.payload).map_err(error))
            .transpose()
    }

    pub fn history(&self) -> Result<Vec<History>> {
        self.query("SELECT event_id,last_edition,last_fingerprint,story FROM events ORDER BY last_edition DESC LIMIT 350",&[])
    }

    pub fn history_before(&self, date: &str) -> Result<Vec<History>> {
        self.query("SELECT event_id,edition_date last_edition,fingerprint last_fingerprint,story FROM (SELECT *,ROW_NUMBER() OVER(PARTITION BY event_id ORDER BY edition_date DESC) position FROM event_history WHERE edition_date<?) WHERE position=1 LIMIT 350", &[date.into()])
    }

    pub fn coverage(&self, previous: i64, cutoff: i64) -> Result<Vec<Coverage>> {
        let rows: Vec<Observation> = self.query("SELECT source_id,status,discovered,detail FROM observations WHERE observed_at>=? AND observed_at<=? ORDER BY observed_at",&[integer(previous),integer(cutoff)])?;
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
            // Daily collection is a single snapshot; it does not promise continuous monitoring.
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

    pub fn latest_run(&self, date: &str) -> Result<Option<Run>> {
        Ok(self
            .query(
                "SELECT * FROM runs WHERE edition_date=? ORDER BY started_at DESC LIMIT 1",
                &[date.into()],
            )?
            .into_iter()
            .next())
    }

    pub fn start_run(&self, id: &str, date: &str, cutoff: i64, started: i64) -> Result<()> {
        self.execute(
            "INSERT INTO runs(id,edition_date,cutoff,started_at,status) VALUES(?,?,?,?,'running')",
            &[id.into(), date.into(), integer(cutoff), integer(started)],
        )
    }

    pub fn fail_run(&self, id: &str, detail: &str) -> Result<()> {
        self.execute("UPDATE runs SET status='incomplete',finished_at=?,detail=? WHERE id=? AND status='running'",&[integer(time::now()),detail.into(),id.into()])
    }

    pub fn publish(
        &self,
        edition: &Edition,
        fingerprints: &[(String, String)],
        run: &str,
    ) -> Result<()> {
        let mut statements = vec![
            Self::statement(
                include_str!("../../sql/publish.sql"),
                &[
                    edition.date.clone().into(),
                    integer(edition.cutoff),
                    integer(edition.generation_started_at),
                    run.into(),
                    integer(edition.cutoff + 3600),
                ],
            ),
            Self::statement(
                include_str!("../../sql/publish_revision.sql"),
                &[
                    edition.date.clone().into(),
                    serde_json::to_string(edition).map_err(error)?.into(),
                ],
            ),
            Self::statement(
                "UPDATE runs SET status='published',finished_at=(SELECT published_at FROM editions WHERE edition_date=?) WHERE id=?",
                &[edition.date.clone().into(), run.into()],
            ),
        ];
        for (event, fingerprint) in fingerprints {
            let story = edition
                .stories
                .iter()
                .find(|s| s.event_id == *event)
                .ok_or_else(|| error("Missing event"))?;
            statements.push(Self::statement("INSERT INTO events(event_id,last_edition,last_fingerprint,story) VALUES(?,?,?,?) ON CONFLICT(event_id) DO UPDATE SET last_edition=excluded.last_edition,last_fingerprint=excluded.last_fingerprint,story=excluded.story WHERE excluded.last_edition>=events.last_edition",&[event.clone().into(),edition.date.clone().into(),fingerprint.clone().into(),serde_json::to_string(story).map_err(error)?.into()]));
        }
        for (event, fingerprint) in fingerprints {
            let story = edition
                .stories
                .iter()
                .find(|s| s.event_id == *event)
                .ok_or_else(|| error("Missing event"))?;
            statements.push(Self::statement("INSERT INTO event_history(event_id,edition_date,fingerprint,story) VALUES(?,?,?,?)", &[event.clone().into(),edition.date.clone().into(),fingerprint.clone().into(),serde_json::to_string(story).map_err(error)?.into()]));
        }
        for evidence in &edition.evidence {
            statements.push(Self::statement(
                "INSERT OR IGNORE INTO published_evidence(id,edition_date) VALUES(?,?)",
                &[evidence.id.clone().into(), edition.date.clone().into()],
            ));
        }
        self.batch(statements)
    }

    pub fn correct(
        &self,
        edition: &Edition,
        previous: u32,
        reason: &str,
        fingerprints: &[(String, String)],
    ) -> Result<()> {
        let mut statements = vec![
            Self::statement(
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
            ),
            Self::statement(
                include_str!("../../sql/correct_pointer.sql"),
                &[
                    edition.revision.into(),
                    edition.date.clone().into(),
                    previous.into(),
                    edition.date.clone().into(),
                    edition.revision.into(),
                ],
            ),
        ];
        let payload = serde_json::to_string(edition).map_err(error)?;
        for (event, fingerprint) in fingerprints {
            let story = edition
                .stories
                .iter()
                .find(|s| s.event_id == *event)
                .ok_or_else(|| error("Missing corrected event"))?;
            let text = serde_json::to_string(story).map_err(error)?;
            statements.push(Self::statement("UPDATE events SET story=?,last_fingerprint=? WHERE event_id=? AND last_edition=? AND EXISTS(SELECT 1 FROM revisions WHERE edition_date=? AND revision=? AND payload=?)", &[text.clone().into(),fingerprint.clone().into(),event.clone().into(),edition.date.clone().into(),edition.date.clone().into(),edition.revision.into(),payload.clone().into()]));
            statements.push(Self::statement("UPDATE event_history SET story=?,fingerprint=? WHERE event_id=? AND edition_date=? AND EXISTS(SELECT 1 FROM revisions WHERE edition_date=? AND revision=? AND payload=?)", &[text.into(),fingerprint.clone().into(),event.clone().into(),edition.date.clone().into(),edition.date.clone().into(),edition.revision.into(),payload.clone().into()]));
        }
        self.batch(statements)
    }
}
