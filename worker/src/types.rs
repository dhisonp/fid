use serde::{Deserialize, Serialize};

macro_rules! text_enum {
    ($name:ident { $($variant:ident => $value:literal),+ $(,)? }) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
        pub enum $name { $(#[serde(rename=$value)] $variant),+ }
        impl AsRef<str> for $name {
            fn as_ref(&self) -> &'static str { match self { $(Self::$variant => $value),+ } }
        }
    };
}
text_enum!(Category {
    Security => "security", IndustryPolicy => "industry-policy",
    DevelopmentInfrastructure => "development-infrastructure", Ai => "ai",
    EngineeringPractice => "engineering-practice", ResearchLectures => "research-lectures",
    SoftwareProducts => "software-products",
});
text_enum!(Label { New => "new", Update => "update", Carryover => "carryover", LateDiscovery => "late-discovery", OlderWork => "older-work" });
text_enum!(ClaimKind { Confirmed => "confirmed", Attributed => "attributed", Inference => "inference", Uncertain => "uncertain" });
text_enum!(PaperStatus { Preprint => "preprint", PeerReviewed => "peer-reviewed", Lecture => "lecture" });

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "precision", content = "value", deny_unknown_fields)]
pub enum SourceDate {
    #[serde(rename = "day")]
    Day(chrono::NaiveDate),
    #[serde(rename = "second")]
    Second(chrono::DateTime<chrono::FixedOffset>),
}
impl SourceDate {
    pub fn value(&self) -> String {
        match self {
            Self::Day(day) => day.to_string(),
            Self::Second(time) => time.to_rfc3339(),
        }
    }
    pub fn earliest_timestamp(&self) -> i64 {
        match self {
            Self::Day(day) => day.and_time(chrono::NaiveTime::MIN).and_utc().timestamp(),
            Self::Second(time) => time.timestamp(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub id: String,
    pub url: String,
    pub source_id: String,
    pub title: String,
    pub published: Option<SourceDate>,
    pub updated: Option<SourceDate>,
    pub first_seen: i64,
    pub retrieved_at: i64,
    pub content_hash: String,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Claim {
    pub text: String,
    pub kind: ClaimKind,
    pub evidence_id: String,
    pub quote: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaperInspection {
    pub version: String,
    pub status: PaperStatus,
    pub methods: String,
    pub results: String,
    pub limitations: String,
    pub artifacts: Vec<String>,
    pub executed: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Story {
    pub event_id: String,
    pub category: Category,
    pub headline: String,
    pub paragraphs: Vec<String>,
    pub claims: Vec<Claim>,
    pub evidence_ids: Vec<String>,
    pub label: Label,
    pub delta: String,
    pub uncertainty: String,
    pub score: u8,
    pub rationale: String,
    pub paper: Option<PaperInspection>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Coverage {
    pub source_id: String,
    pub checks: usize,
    pub successful_checks: usize,
    pub discovered: i64,
    pub gaps: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Correction {
    pub at: i64,
    pub reason: String,
    pub previous_revision: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Edition {
    #[serde(default)]
    pub verification: bool,
    pub date: String,
    pub revision: u32,
    pub cutoff: i64,
    pub generation_started_at: i64,
    pub published_at: i64,
    pub stories: Vec<Story>,
    pub evidence: Vec<Evidence>,
    pub coverage: Vec<Coverage>,
    pub corrections: Vec<Correction>,
}

// Reader responses exclude the verification ledger and archived document contents.
#[derive(Serialize)]
pub struct PublicEdition<'a> {
    pub verification: bool,
    pub date: &'a str,
    pub revision: u32,
    pub cutoff: i64,
    pub generation_started_at: i64,
    pub published_at: i64,
    pub stories: Vec<PublicStory<'a>>,
    pub evidence: Vec<PublicSource<'a>>,
    pub coverage: &'a [Coverage],
    pub corrections: &'a [Correction],
}

#[derive(Serialize)]
pub struct PublicStory<'a> {
    pub event_id: &'a str,
    pub category: Category,
    pub headline: &'a str,
    pub paragraphs: &'a [String],
    pub evidence_ids: &'a [String],
    pub label: Label,
    pub delta: &'a str,
    pub uncertainty: &'a str,
}

#[derive(Serialize)]
pub struct PublicSource<'a> {
    pub id: &'a str,
    pub url: &'a str,
    pub published: &'a Option<SourceDate>,
}

impl<'a> From<&'a Edition> for PublicEdition<'a> {
    fn from(edition: &'a Edition) -> Self {
        Self {
            verification: edition.verification,
            date: &edition.date,
            revision: edition.revision,
            cutoff: edition.cutoff,
            generation_started_at: edition.generation_started_at,
            published_at: edition.published_at,
            stories: edition
                .stories
                .iter()
                .map(|story| PublicStory {
                    event_id: &story.event_id,
                    category: story.category,
                    headline: &story.headline,
                    paragraphs: &story.paragraphs,
                    evidence_ids: &story.evidence_ids,
                    label: story.label,
                    delta: &story.delta,
                    uncertainty: &story.uncertainty,
                })
                .collect(),
            evidence: edition
                .evidence
                .iter()
                .map(|source| PublicSource {
                    id: &source.id,
                    url: &source.url,
                    published: &source.published,
                })
                .collect(),
            coverage: &edition.coverage,
            corrections: &edition.corrections,
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Draft {
    pub stories: Vec<Story>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Audit {
    pub approved_events: Vec<String>,
    pub issues: Vec<String>,
}

#[derive(Deserialize, Serialize)]
pub struct History {
    pub event_id: String,
    pub last_edition: String,
    pub last_fingerprint: String,
    pub story: String,
}

#[derive(Deserialize, Serialize)]
pub struct Run {
    pub id: String,
    pub edition_date: String,
    pub cutoff: i64,
    pub started_at: i64,
    pub finished_at: Option<i64>,
    pub status: String,
    pub detail: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn public_edition_excludes_internal_evidence_without_changing_archive() {
        let stored = json!({
            "verification":false,"date":"2026-10-23","revision":1,
            "cutoff":200,"generation_started_at":201,"published_at":202,
            "stories":[{
                "event_id":"event","category":"security","headline":"A material change",
                "paragraphs":["Affected scope and mitigation."],
                "claims":[{"text":"Affected scope","kind":"confirmed","evidence_id":"source","quote":"An exact supporting quotation from the source."}],
                "evidence_ids":["source"],"label":"new","delta":"","uncertainty":"",
                "score":80,"rationale":"Technical consequence.",
                "paper":{"version":"v1","status":"preprint","methods":"Inspected methods.",
                    "results":"Reported results.","limitations":"Known limitations.",
                    "artifacts":[],"executed":false}
            }],
            "evidence":[{
                "id":"source","url":"https://example.com/advisory","source_id":"feed",
                "title":"Advisory","published":{"precision":"day","value":"2026-10-22"},
                "updated":null,"first_seen":100,"retrieved_at":150,
                "content_hash":"hash","text":"Full archived source text."
            }],
            "coverage":[],"corrections":[]
        });
        let edition: Edition = serde_json::from_value(stored.clone()).unwrap();
        let public = serde_json::to_value(PublicEdition::from(&edition)).unwrap();
        let story = &public["stories"][0];
        for field in ["claims", "paper", "score", "rationale"] {
            assert!(story.get(field).is_none());
        }
        for field in [
            "event_id",
            "category",
            "headline",
            "paragraphs",
            "evidence_ids",
            "label",
            "delta",
            "uncertainty",
        ] {
            assert_eq!(story[field], stored["stories"][0][field]);
        }
        assert_eq!(
            public["evidence"],
            json!([{
                "id":"source","url":"https://example.com/advisory",
                "published":{"precision":"day","value":"2026-10-22"}
            }])
        );
        assert_eq!(serde_json::to_value(&edition).unwrap(), stored);
    }
}
