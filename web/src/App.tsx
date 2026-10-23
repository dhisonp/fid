import { Fragment, useEffect, useRef, useState } from 'react';
import type { KeyboardEvent } from 'react';
import { categories, loadFeed } from './api';
import type {
  Category,
  Edition,
  Evidence,
  Feed,
  SourceDate,
  Story,
} from './api';

const timestamp = (value: number) =>
  new Intl.DateTimeFormat('en-US', {
    timeZone: 'America/New_York',
    month: 'short',
    day: 'numeric',
    hour: 'numeric',
    minute: '2-digit',
    timeZoneName: 'short',
  }).format(value * 1000);
const day = (value: string) =>
  new Intl.DateTimeFormat('en-US', {
    timeZone: 'UTC',
    weekday: 'long',
    year: 'numeric',
    month: 'long',
    day: 'numeric',
  }).format(new Date(`${value}T12:00:00Z`));
function date(value: SourceDate | null) {
  if (value === null) return 'Publication date unknown';
  return value.precision === 'day'
    ? day(value.value)
    : timestamp(Date.parse(value.value) / 1000);
}
function Inline({ text }: { text: string }) {
  return (
    <>
      {text
        .split(/(`[^`]+`)/g)
        .map((part, i) =>
          part.startsWith('`') && part.endsWith('`') ? (
            <code key={i}>{part.slice(1, -1)}</code>
          ) : (
            <Fragment key={i}>{part}</Fragment>
          ),
        )}
    </>
  );
}
function preferences(): Category[] {
  try {
    const raw: unknown = JSON.parse(
      localStorage.getItem('fid.categories') ?? 'null',
    );
    if (Array.isArray(raw))
      return categories.map((c) => c[0]).filter((id) => raw.includes(id));
  } catch {
    /* Storage may be unavailable; default display remains usable. */
  }
  return categories.map((c) => c[0]);
}
function Source({ source }: { source: Evidence }) {
  return (
    <li>
      <a href={source.url} rel="noreferrer">
        <Inline text={source.title || new URL(source.url).hostname} />
      </a>
      <span className="source-date">
        Published: {date(source.published)}
        {source.updated && <> · Updated: {date(source.updated)}</>}
      </span>
      <span className="source-date">
        First seen: {timestamp(source.first_seen)} · Retrieved:{' '}
        {timestamp(source.retrieved_at)}
      </span>
      <a className="archive-link" href={`/api/evidence/${source.id}`}>
        Inspect archived evidence
      </a>
    </li>
  );
}
function StoryCard({ story, edition }: { story: Story; edition: Edition }) {
  const sources = edition.evidence.filter((e) =>
    story.evidence_ids.includes(e.id),
  );
  const label = {
    new: 'New',
    update: 'Material update',
    carryover: 'Carryover',
    'late-discovery': 'Late discovery',
    'older-work': 'Older work',
  }[story.label];
  return (
    <article className="story" id={story.event_id}>
      <p className="story-date">
        <span className="label">{label}</span> ·{' '}
        {date(sources[0]?.published ?? null)}
      </p>
      <h3>
        <a className="story-anchor" href={`#${story.event_id}`}>
          <Inline text={story.headline} />
        </a>
      </h3>
      {story.delta && (
        <p className="delta">
          <strong>What changed:</strong> <Inline text={story.delta} />
        </p>
      )}
      {story.paragraphs.map((text, i) => (
        <p key={i}>
          <Inline text={text} />
        </p>
      ))}
      {story.uncertainty && (
        <p>
          <strong>Unresolved:</strong> <Inline text={story.uncertainty} />
        </p>
      )}
      <div className="sources">
        <p>Original sources</p>
        <ul>
          {sources.map((source) => (
            <Source key={source.id} source={source} />
          ))}
        </ul>
      </div>
      <details className="evidence">
        <summary>Evidence behind the reporting</summary>
        {story.claims.map((claim, i) => (
          <div className="claim" key={i}>
            <p>
              <span className="label">{claim.kind}</span>{' '}
              <Inline text={claim.text} />
            </p>
            <blockquote>{claim.quote}</blockquote>
            <a href={`/api/evidence/${claim.evidence_id}`}>Archived source</a>
          </div>
        ))}
        {story.paper && (
          <div className="paper">
            <h4>Research inspection</h4>
            <p>
              {story.paper.version} · {story.paper.status}
            </p>
            <p>
              <strong>Methods:</strong> {story.paper.methods}
            </p>
            <p>
              <strong>Results:</strong> {story.paper.results}
            </p>
            <p>
              <strong>Limitations:</strong> {story.paper.limitations}
            </p>
            <p>
              Artifacts were not executed; results have not been independently
              reproduced by Fid.
            </p>
            {story.paper.artifacts.map((url) => (
              <p key={url}>
                <a href={url} rel="noreferrer">
                  Research artifact: {new URL(url).hostname}
                </a>
              </p>
            ))}
          </div>
        )}
      </details>
    </article>
  );
}
function EditionNotes({ edition }: { edition: Edition }) {
  const gaps = edition.coverage.filter((c) => c.gaps.length > 0);
  return (
    <>
      {edition.corrections.length > 0 && (
        <aside className="notice" aria-label="Corrections">
          <strong>Corrections to Fid’s reporting</strong>
          {edition.corrections.map((c) => (
            <p key={c.at}>
              {timestamp(c.at)} — {c.reason}{' '}
              <a
                href={`/editions/${edition.date}/revisions/${c.previous_revision}`}
              >
                Read earlier revision
              </a>
            </p>
          ))}
        </aside>
      )}
      <details className="edition-notes">
        <summary>
          Collection & evidence ·{' '}
          {gaps.length === 0
            ? 'No observed coverage gaps'
            : `${gaps.length} sources with coverage gaps`}
        </summary>
        <dl>
          <dt>Collection cutoff</dt>
          <dd>{timestamp(edition.cutoff)}</dd>
          <dt>Generation started</dt>
          <dd>{timestamp(edition.generation_started_at)}</dd>
          <dt>Published</dt>
          <dd>{timestamp(edition.published_at)}</dd>
        </dl>
        <p>
          Coverage reflects the checks actually observed. Hacker News and search
          are discovery channels; originating documents support the reporting.
        </p>
        <ul className="coverage">
          {edition.coverage.map((c) => (
            <li key={c.source_id}>
              <strong>{c.source_id}</strong> — {c.successful_checks}/{c.checks}{' '}
              successful checks; {c.discovered} documents collected.
              {c.gaps.map((gap, i) => (
                <p key={i}>{gap}</p>
              ))}
            </li>
          ))}
        </ul>
      </details>
    </>
  );
}
function feedPath(): string {
  const match =
    /^\/editions\/(\d{4}-\d{2}-\d{2})(?:\/revisions\/(\d+))?\/?$/.exec(
      location.pathname,
    );
  if (match)
    return `/api/editions/${match[1]}${match[2] ? `/revisions/${match[2]}` : ''}`;
  return location.pathname === '/' ? '/api/latest' : '/api/not-found';
}

export default function App() {
  const [feed, setFeed] = useState<Feed | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [attempt, setAttempt] = useState(0);
  const [visible, setVisible] = useState<Category[]>(preferences);
  const [active, setActive] = useState<Category>(visible[0] ?? 'security');
  const [storageNotice, setStorageNotice] = useState(false);
  const tabs = useRef<(HTMLButtonElement | null)[]>([]);
  const shown = categories.filter((c) => visible.includes(c[0]));
  useEffect(() => {
    const controller = new AbortController();
    loadFeed(feedPath(), controller.signal)
      .then(setFeed)
      .catch((failure: unknown) => {
        if (!controller.signal.aborted)
          setError(
            failure instanceof Error
              ? failure.message
              : 'Unable to load the briefing.',
          );
      });
    return () => controller.abort();
  }, [attempt]);
  useEffect(() => {
    // Update stale state after the deadline even when a reader leaves the page open.
    const timer = window.setInterval(() => {
      setError(null);
      setAttempt((a) => a + 1);
    }, 5 * 60_000);
    return () => window.clearInterval(timer);
  }, []);
  function toggle(id: Category) {
    const next = visible.includes(id)
      ? visible.filter((c) => c !== id)
      : [...visible, id];
    setVisible(next);
    if (!next.includes(active))
      setActive(categories.find((c) => next.includes(c[0]))?.[0] ?? 'security');
    try {
      localStorage.setItem('fid.categories', JSON.stringify(next));
    } catch {
      setStorageNotice(true);
    }
  }
  function tabKey(event: KeyboardEvent<HTMLButtonElement>, index: number) {
    const next =
      event.key === 'ArrowRight'
        ? (index + 1) % shown.length
        : event.key === 'ArrowLeft'
          ? (index - 1 + shown.length) % shown.length
          : event.key === 'Home'
            ? 0
            : event.key === 'End'
              ? shown.length - 1
              : null;
    if (next !== null) {
      event.preventDefault();
      setActive(shown[next][0]);
      tabs.current[next]?.focus();
    }
  }
  const edition = feed?.edition ?? null;
  return (
    <div className="page">
      <a className="skip" href="#reading">
        Skip to briefing
      </a>
      <header className="masthead">
        <div>
          <a className="wordmark" href="/">
            Fid
          </a>
          <h1>Daily technical briefing</h1>
          <p className="date">
            {edition ? (
              <time dateTime={edition.date}>{day(edition.date)}</time>
            ) : (
              'A considered view of what changed.'
            )}
          </p>
        </div>
        <p className="schedule">
          Every morning, by 7:30 a.m.
          <br />
          New York time
        </p>
      </header>
      <div aria-live="polite">
        {error && (
          <aside className="notice" role="alert">
            <p>{error}</p>
            <button
              onClick={() => {
                setError(null);
                setAttempt((a) => a + 1);
              }}
            >
              Try again
            </button>
          </aside>
        )}
        {feed === null && error === null && (
          <p className="empty">Loading the briefing…</p>
        )}
        {feed?.status === 'stale' && (
          <aside className="notice">
            <strong>The latest edition is delayed.</strong>
            <p>
              Showing the complete {edition?.date} edition. The{' '}
              {feed.expected_date} edition has not passed publication checks.
            </p>
          </aside>
        )}
        {feed?.run?.status === 'incomplete' && feed.status !== 'available' && (
          <p className="status">Publication incomplete. {feed.run.detail}</p>
        )}
      </div>
      {feed?.status === 'unavailable' && (
        <main id="reading" className="empty">
          <h2>No edition is available yet.</h2>
          <p>
            Fid publishes only when every category has enough substantive,
            supported reporting. Collection and publication run independently of
            your visit.
          </p>
          <p>The next complete edition will appear here automatically.</p>
        </main>
      )}
      {edition && (
        <>
          {edition.verification && (
            <aside className="notice">
              <strong>Local verification edition</strong>
              <p>
                This edition uses real collected evidence and an off-schedule
                cutoff for local testing. It is not a scheduled daily
                publication.
              </p>
            </aside>
          )}
          <div className="edition-bar">
            <p>
              Published {timestamp(edition.published_at)} · Cutoff{' '}
              {timestamp(edition.cutoff)} · Revision {edition.revision}
            </p>
            <a href={`/editions/${edition.date}/revisions/${edition.revision}`}>
              Edition permalink
            </a>
          </div>
          <EditionNotes edition={edition} />
          <details className="display">
            <summary>
              Choose visible categories <span>{visible.length} of 7</span>
            </summary>
            <fieldset>
              <legend>Display preferences · saved on this device</legend>
              {categories.map(([id, name]) => (
                <label key={id}>
                  <input
                    type="checkbox"
                    checked={visible.includes(id)}
                    onChange={() => toggle(id)}
                  />
                  {name}
                </label>
              ))}
            </fieldset>
            {storageNotice && (
              <p>
                Your browser could not save this preference. Your current
                selection still applies.
              </p>
            )}
          </details>
          <nav className="tabs" role="tablist" aria-label="News categories">
            {shown.map(([id, name], i) => (
              <button
                key={id}
                ref={(el) => {
                  tabs.current[i] = el;
                }}
                role="tab"
                id={`tab-${id}`}
                aria-controls={`panel-${id}`}
                aria-selected={active === id}
                tabIndex={active === id ? 0 : -1}
                onClick={() => setActive(id)}
                onKeyDown={(event) => tabKey(event, i)}
              >
                {name}
              </button>
            ))}
          </nav>
          <main className="columns" id="reading" tabIndex={-1}>
            {shown.length === 0 && (
              <p className="empty">
                All categories are hidden. Choose a category above to start
                reading.
              </p>
            )}
            {shown.map(([id, name]) => (
              <section
                className={`column ${active === id ? 'active' : ''}`}
                key={id}
                role="tabpanel"
                id={`panel-${id}`}
                aria-labelledby={`tab-${id}`}
                tabIndex={0}
              >
                <div className="column-heading">
                  <h2>{name}</h2>
                  <span>
                    {edition.stories.filter((s) => s.category === id).length}{' '}
                    entries
                  </span>
                </div>
                {edition.stories
                  .filter((s) => s.category === id)
                  .map((story) => (
                    <StoryCard
                      key={story.event_id}
                      story={story}
                      edition={edition}
                    />
                  ))}
              </section>
            ))}
          </main>
        </>
      )}
      <footer>
        <p>Fid · Read the change. Inspect the evidence.</p>
        <p>
          One shared edition. Your category choices change only the display.
        </p>
        <a href="/api/editions">Published edition archive</a>
      </footer>
    </div>
  );
}
