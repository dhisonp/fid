use serde::Serialize;

#[derive(Clone, Copy, Serialize)]
pub struct Source {
    pub id: &'static str,
    pub url: &'static str,
    pub category: &'static str,
}

pub const SOURCES: &[Source] = &[
    Source {
        id: "cisa",
        url: "https://www.cisa.gov/cybersecurity-advisories/all.xml",
        category: "security",
    },
    Source {
        id: "google-security",
        url: "https://security.googleblog.com/feeds/posts/default",
        category: "security",
    },
    Source {
        id: "project-zero",
        url: "https://googleprojectzero.blogspot.com/feeds/posts/default",
        category: "security",
    },
    Source {
        id: "ftc",
        url: "https://www.ftc.gov/feeds/press-release.xml",
        category: "industry-policy",
    },
    Source {
        id: "eff",
        url: "https://www.eff.org/rss/updates.xml",
        category: "industry-policy",
    },
    Source {
        id: "github-blog",
        url: "https://github.blog/feed/",
        category: "software-products",
    },
    Source {
        id: "rust",
        url: "https://blog.rust-lang.org/feed.xml",
        category: "development-infrastructure",
    },
    Source {
        id: "postgresql",
        url: "https://www.postgresql.org/news.rss",
        category: "development-infrastructure",
    },
    Source {
        id: "cloudflare",
        url: "https://blog.cloudflare.com/rss/",
        category: "development-infrastructure",
    },
    Source {
        id: "kubernetes",
        url: "https://kubernetes.io/feed.xml",
        category: "development-infrastructure",
    },
    Source {
        id: "openai",
        url: "https://openai.com/news/rss.xml",
        category: "ai",
    },
    Source {
        id: "google-research",
        url: "https://research.google/blog/rss/",
        category: "ai",
    },
    Source {
        id: "huggingface",
        url: "https://huggingface.co/blog/feed.xml",
        category: "ai",
    },
    Source {
        id: "aws-builders",
        url: "https://aws.amazon.com/builders-library/feed/",
        category: "engineering-practice",
    },
    Source {
        id: "netflix-tech",
        url: "https://netflixtechblog.com/feed",
        category: "engineering-practice",
    },
    Source {
        id: "brendan-gregg",
        url: "https://www.brendangregg.com/blog/rss.xml",
        category: "engineering-practice",
    },
    Source {
        id: "arxiv-systems",
        url: "https://rss.arxiv.org/rss/cs.DC+cs.DB+cs.OS",
        category: "research-lectures",
    },
    Source {
        id: "acm-queue",
        url: "https://queue.acm.org/rss/feeds/queuecontent.xml",
        category: "research-lectures",
    },
    Source {
        id: "simon-willison",
        url: "https://simonwillison.net/atom/everything/",
        category: "software-products",
    },
    Source {
        id: "vscode",
        url: "https://code.visualstudio.com/feed.xml",
        category: "software-products",
    },
];
