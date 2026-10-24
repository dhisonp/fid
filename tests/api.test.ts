import { test } from 'node:test';
import assert from 'node:assert/strict';
import { parseFeed } from '../web/src/api.ts';

function feed() {
  return {
    status: 'unavailable',
    expected_date: '2026-10-23',
    checked_at: 1792755000,
    edition: null,
    run: null,
  };
}
function edition() {
  return {
    verification: false,
    date: '2026-10-23',
    revision: 1,
    cutoff: 1792751400,
    generation_started_at: 1792751500,
    published_at: 1792752000,
    stories: [],
    evidence: [],
    coverage: [],
    corrections: [],
  };
}
test('first publication failure is an honest empty state', () => {
  assert.equal(parseFeed(feed()).edition, null);
  assert.equal(parseFeed(feed()).status, 'unavailable');
});
test('a stale response retains the previous edition and failure details', () => {
  const value = parseFeed({
    ...feed(),
    status: 'stale',
    edition: edition(),
    run: {
      status: 'incomplete',
      detail: 'Insufficient qualifying stories',
      started_at: 1792751500,
    },
  });
  assert.equal(value.edition?.date, '2026-10-23');
  assert.equal(value.run?.status, 'incomplete');
});
test('corrections and original revisions remain visible data', () => {
  const e = {
    ...edition(),
    revision: 2,
    corrections: [
      {
        at: 1792753000,
        reason: 'Corrected affected version from originating advisory.',
        previous_revision: 1,
      },
    ],
  };
  assert.equal(
    parseFeed({ ...feed(), status: 'available', edition: e }).edition
      ?.corrections[0].previous_revision,
    1,
  );
});
test('unknown response states and malformed timestamps are rejected at the boundary', () => {
  assert.throws(() => parseFeed({ ...feed(), status: 'probably-fine' }));
  assert.throws(() => parseFeed({ ...feed(), checked_at: 'yesterday' }));
  assert.throws(() => parseFeed({ ...feed(), checked_at: 1e25 }));
  assert.throws(() => parseFeed({ ...feed(), expected_date: '2026-02-30' }));
});
test('source dates retain precision; executable source URLs are rejected', () => {
  const evidence = {
    id: 'source',
    url: 'https://example.com/advisory',
    published: { value: '2026-10-22', precision: 'day' },
  };
  const value = parseFeed({
    ...feed(),
    status: 'available',
    edition: { ...edition(), evidence: [evidence] },
  });
  assert.equal(value.edition?.evidence[0].published?.precision, 'day');
  assert.throws(() =>
    parseFeed({
      ...feed(),
      edition: {
        ...edition(),
        evidence: [{ ...evidence, url: 'javascript:alert(1)' }],
      },
    }),
  );
});

test('reader feeds need only source links and dates, without verification fields', () => {
  const story = {
    event_id: 'event',
    headline: 'A material change',
    paragraphs: ['Affected scope and mitigation.'],
    evidence_ids: ['source'],
    label: 'new',
    delta: '',
    uncertainty: '',
  };
  const source = {
    id: 'source',
    url: 'https://example.com/advisory',
    published: null,
  };
  const response = {
    ...feed(),
    status: 'available',
    edition: { ...edition(), stories: [story], evidence: [source] },
  };
  const parsed = parseFeed(response);
  assert.deepEqual(parsed.edition?.stories, [story]);
  assert.deepEqual(parsed.edition?.evidence, [source]);
  const legacy = parseFeed({
    ...response,
    edition: {
      ...response.edition,
      stories: [{ ...story, category: 'security' }],
    },
  });
  assert.deepEqual(legacy.edition?.stories, [story]);
  assert.throws(
    () =>
      parseFeed({
        ...response,
        edition: { ...response.edition, evidence: [] },
      }),
    /missing evidence/,
  );
});

test('the unified feed preserves the relevance order across all nine stories', () => {
  const stories = Array.from({ length: 9 }, (_, i) => ({
    event_id: `event-${i}`,
    headline: `Relevant story ${i}`,
    paragraphs: ['A consequential development for engineering leads.'],
    evidence_ids: ['source'],
    label: 'new',
    delta: '',
    uncertainty: '',
  }));
  const parsed = parseFeed({
    ...feed(),
    status: 'available',
    edition: {
      ...edition(),
      stories,
      evidence: [{ id: 'source', url: 'https://example.com', published: null }],
    },
  });
  assert.deepEqual(parsed.edition?.stories, stories);
});
