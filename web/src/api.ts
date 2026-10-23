export const categories = [
  ['security', 'Security'],
  ['industry-policy', 'Industry & Policy'],
  ['development-infrastructure', 'Development & Infrastructure'],
  ['ai', 'AI'],
  ['engineering-practice', 'Engineering Practice'],
  ['research-lectures', 'Research & Lectures'],
  ['software-products', 'Software & Products'],
] as const;
export type Category = (typeof categories)[number][0];
export type SourceDate = { value: string; precision: 'day' | 'second' };
export type Evidence = {
  id: string;
  url: string;
  published: SourceDate | null;
};
export type Story = {
  event_id: string;
  category: Category;
  headline: string;
  paragraphs: string[];
  label: 'new' | 'update' | 'carryover' | 'late-discovery' | 'older-work';
  delta: string;
  uncertainty: string;
  evidence_ids: string[];
};
export type Edition = {
  verification: boolean;
  date: string;
  revision: number;
  cutoff: number;
  generation_started_at: number;
  published_at: number;
  stories: Story[];
  evidence: Evidence[];
  coverage: {
    source_id: string;
    checks: number;
    successful_checks: number;
    discovered: number;
    gaps: string[];
  }[];
  corrections: { at: number; reason: string; previous_revision: number }[];
};
export type Feed = {
  status: 'available' | 'unavailable' | 'stale';
  expected_date: string;
  checked_at: number;
  edition: Edition | null;
  run: null | { status: string; detail: string | null; started_at: number };
};

function object(raw: unknown): Record<string, unknown> {
  if (typeof raw !== 'object' || raw === null || Array.isArray(raw))
    throw new Error('Invalid briefing response.');
  return raw as Record<string, unknown>;
}
function string(raw: unknown): string {
  if (typeof raw !== 'string')
    throw new Error('Invalid text in briefing response.');
  return raw;
}
function number(raw: unknown): number {
  if (typeof raw !== 'number' || !Number.isFinite(raw))
    throw new Error('Invalid date or count in briefing response.');
  return raw;
}
function timestamp(raw: unknown): number {
  const value = number(raw);
  if (!Number.isSafeInteger(value) || Math.abs(value) > 8_640_000_000_000)
    throw new Error('Invalid timestamp in briefing response.');
  return value;
}
function isoDay(raw: unknown): string {
  const value = string(raw);
  if (
    !/^\d{4}-\d{2}-\d{2}$/.test(value) ||
    !Number.isFinite(Date.parse(value)) ||
    new Date(`${value}T00:00:00Z`).toISOString().slice(0, 10) !== value
  )
    throw new Error('Invalid publication date.');
  return value;
}
function array<T>(raw: unknown, parse: (raw: unknown) => T): T[] {
  if (!Array.isArray(raw))
    throw new Error('Invalid list in briefing response.');
  return raw.map(parse);
}
function choice<T extends string>(raw: unknown, values: readonly T[]): T {
  const value = string(raw);
  const result = values.find((v) => v === value);
  if (result === undefined)
    throw new Error('Unknown value in briefing response.');
  return result;
}
function sourceDate(raw: unknown): SourceDate | null {
  if (raw === null) return null;
  const date = object(raw);
  const precision = choice(date.precision, ['day', 'second']);
  const value = precision === 'day' ? isoDay(date.value) : string(date.value);
  if (precision === 'second' && !Number.isFinite(Date.parse(value)))
    throw new Error('Invalid publication timestamp.');
  return { value, precision };
}
function https(raw: unknown): string {
  const value = string(raw);
  if (new URL(value).protocol !== 'https:')
    throw new Error('Invalid source link.');
  return value;
}
function parseEvidence(raw: unknown): Evidence {
  const e = object(raw);
  return {
    id: string(e.id),
    url: https(e.url),
    published: sourceDate(e.published),
  };
}
function parseStory(raw: unknown): Story {
  const s = object(raw);
  return {
    event_id: string(s.event_id),
    category: choice(
      s.category,
      categories.map((c) => c[0]),
    ),
    headline: string(s.headline),
    paragraphs: array(s.paragraphs, string),
    label: choice(s.label, [
      'new',
      'update',
      'carryover',
      'late-discovery',
      'older-work',
    ]),
    delta: string(s.delta),
    uncertainty: string(s.uncertainty),
    evidence_ids: array(s.evidence_ids, string),
  };
}
export function parseFeed(raw: unknown): Feed {
  const f = object(raw);
  const e = f.edition === null ? null : object(f.edition);
  const r = f.run === null ? null : object(f.run);
  const edition: Edition | null =
    e === null
      ? null
      : {
          verification: e.verification === true,
          date: isoDay(e.date),
          revision: number(e.revision),
          cutoff: timestamp(e.cutoff),
          generation_started_at: timestamp(e.generation_started_at),
          published_at: timestamp(e.published_at),
          stories: array(e.stories, parseStory),
          evidence: array(e.evidence, parseEvidence),
          coverage: array(e.coverage, (raw) => {
            const c = object(raw);
            return {
              source_id: string(c.source_id),
              checks: number(c.checks),
              successful_checks: number(c.successful_checks),
              discovered: number(c.discovered),
              gaps: array(c.gaps, string),
            };
          }),
          corrections: array(e.corrections, (raw) => {
            const c = object(raw);
            return {
              at: timestamp(c.at),
              reason: string(c.reason),
              previous_revision: number(c.previous_revision),
            };
          }),
        };
  if (edition !== null) {
    const ids = new Set(edition.evidence.map((e) => e.id));
    if (edition.stories.some((s) => s.evidence_ids.some((id) => !ids.has(id))))
      throw new Error('Briefing references missing evidence.');
  }
  return {
    status: choice(f.status, ['available', 'unavailable', 'stale']),
    expected_date: isoDay(f.expected_date),
    checked_at: timestamp(f.checked_at),
    edition,
    run:
      r === null
        ? null
        : {
            status: choice(r.status, ['running', 'incomplete', 'published']),
            detail: r.detail === null ? null : string(r.detail),
            started_at: timestamp(r.started_at),
          },
  };
}
export async function loadFeed(
  path: string,
  signal: AbortSignal,
): Promise<Feed> {
  const response = await fetch(path, { signal });
  if (!response.ok)
    throw new Error(
      response.status === 404
        ? 'That edition is not available.'
        : 'The briefing could not be loaded. Please try again.',
    );
  return parseFeed(await response.json());
}
