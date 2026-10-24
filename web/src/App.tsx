import { Fragment, useEffect, useState } from 'react';
import { loadFeed } from './api';
import type { Edition, Feed, SourceDate, Story } from './api';

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
function StoryCard({ story, edition }: { story: Story; edition: Edition }) {
  const sources = edition.evidence.filter((e) =>
    story.evidence_ids.includes(e.id),
  );
  const label = {
    new: 'New',
    update: 'Material update',
    carryover: 'Carryover',
    'late-discovery': 'Late discovery',
    'older-work': null,
  }[story.label];
  return (
    <article className="story" id={story.event_id}>
      <p className="story-date">
        {label && (
          <>
            <span className="label">{label}</span> ·{' '}
          </>
        )}
        {date(sources[0]?.published ?? null)}
      </p>
      <h2>
        <a className="story-anchor" href={`#${story.event_id}`}>
          <Inline text={story.headline} />
        </a>
      </h2>
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
      <ul className="sources" aria-label="Sources">
        {sources.map((source) => (
          <li key={source.id}>
            <a href={source.url} target="_blank" rel="noopener noreferrer">
              {new URL(source.url).hostname}
            </a>
          </li>
        ))}
      </ul>
    </article>
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
  const edition = feed?.edition ?? null;
  return (
    <div className="page">
      <a className="skip" href="#reading">
        Skip to briefing
      </a>
      <header className="masthead">
        <h1>
          <a className="wordmark" href="/">
            Fid
          </a>
        </h1>
        {edition && (
          <p className="date">
            <time dateTime={edition.date}>{day(edition.date)}</time>
          </p>
        )}
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
        {feed?.run?.status === 'running' && (
          <p className="status" role="status">
            The next briefing is being prepared. It will appear after
            publication checks finish.
          </p>
        )}
      </div>
      {feed?.status === 'unavailable' && (
        <main id="reading" className="empty">
          <h2>No edition is available yet.</h2>
          <p>
            Fid publishes nine relevant stories backed by substantive, supported
            reporting. Collection and publication run independently of your
            visit.
          </p>
          <p>The next complete edition will appear here automatically.</p>
        </main>
      )}
      {edition && (
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
          <main className="reading" id="reading" tabIndex={-1}>
            {edition.stories.map((story) => (
              <StoryCard key={story.event_id} story={story} edition={edition} />
            ))}
          </main>
        </>
      )}
    </div>
  );
}
