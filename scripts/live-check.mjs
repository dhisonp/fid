import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';

const origin = process.env.FID_ORIGIN ?? 'http://127.0.0.1:8787';
const token = process.env.ADMIN_TOKEN;
if (!['localhost', '127.0.0.1', '[::1]'].includes(new URL(origin).hostname))
  throw new Error('Live verification requires a local Worker.');
if (!token || token.length < 24)
  throw new Error(
    'Set ADMIN_TOKEN in your shell and .dev.vars (minimum 24 characters).',
  );
async function request(path, method = 'GET') {
  const response = await fetch(`${origin}${path}`, {
    method,
    headers: { Authorization: `Bearer ${token}` },
    signal: AbortSignal.timeout(840_000),
  });
  if (!response.ok)
    throw new Error(
      `${method} ${path}: HTTP ${response.status}: ${await response.text()}`,
    );
  return response.json();
}
await request('/api/admin/collect', 'POST');
const feed = await request('/api/admin/verify', 'POST');
assert.equal(feed.status, 'available');
assert.equal(feed.edition.verification, true);
const { edition } = feed;
assert(edition.evidence.length > 0);
assert(
  new Set(edition.stories.map((s) => s.event_id)).size ===
    edition.stories.length,
);
const ids = [
  'security',
  'industry-policy',
  'development-infrastructure',
  'ai',
  'engineering-practice',
  'research-lectures',
  'software-products',
];
for (const id of ids) {
  const count = edition.stories.filter((s) => s.category === id).length;
  assert(count >= 4 && count <= 7, `${id}: ${count}`);
}
for (const evidence of edition.evidence) {
  assert(evidence.retrieved_at <= edition.cutoff);
  assert.equal(new URL(evidence.url).protocol, 'https:');
  const archived = await request(`/api/evidence/${evidence.id}`);
  assert(archived.text.length >= 300);
  for (const story of edition.stories) {
    for (const claim of story.claims.filter(
      (c) => c.evidence_id === evidence.id,
    ))
      assert(archived.text.includes(claim.quote));
  }
}
const saved = await request(`/api/editions/${edition.date}`);
assert.equal(saved.edition.published_at, edition.published_at);
const retry = await fetch(`${origin}/api/admin/verify`, {
  method: 'POST',
  headers: { Authorization: `Bearer ${token}` },
});
assert.equal(retry.status, 409);
assert.equal(
  (await request(`/api/editions/${edition.date}`)).edition.published_at,
  edition.published_at,
);
const usage = await request('/api/admin/usage');
await mkdir('artifacts', { recursive: true });
await writeFile(
  'artifacts/live-verification.json',
  JSON.stringify(
    { checked_at: new Date().toISOString(), edition, usage },
    null,
    2,
  ),
);
console.log(
  `Real-source flow verified: ${edition.stories.length} stories; ${edition.evidence.length} documents. Read ${origin}/editions/${edition.date}`,
);
