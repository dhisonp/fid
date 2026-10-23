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
