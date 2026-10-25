import json
import sqlite3
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


class StorageTests(unittest.TestCase):
    def setUp(self):
        self.db = sqlite3.connect(':memory:')
        self.clock = 103
        self.db.create_function('unixepoch', 0, lambda: self.clock)
        self.db.executescript((ROOT / 'migrations/0001_initial.sql').read_text())

    def tearDown(self):
        self.db.close()

    def sql(self, name):
        return (ROOT / 'sql' / name).read_text()

    def lease(self, owner='run-a', now=100):
        return self.db.execute(
            self.sql('acquire.sql'), ('generation', owner, now + 840, now)
        ).fetchall()

    def publish(self, date='2026-10-23', owner='run-a', broken=False):
        with self.db:
            self.db.execute(self.sql('publish.sql'), (date, 100, 101, owner, 3700))
            self.db.execute(
                self.sql('publish_revision.sql'),
                (date, '{broken' if broken else json.dumps({'date': date})),
            )
            # The runtime's remaining event/reference writes enforce the edition FK.
            self.db.execute(
                'INSERT INTO event_history VALUES(?,?,?,?)',
                ('event', date, 'hash', '{}'),
            )

    def test_lease_excludes_concurrent_jobs_and_expires(self):
        self.assertEqual(self.lease(), [('run-a',)])
        self.assertEqual(self.lease('run-b', 200), [])
        self.assertEqual(self.lease('run-b', 941), [('run-b',)])

    def test_retry_cannot_duplicate_or_regress_edition(self):
        self.lease()
        self.publish()
        with self.assertRaises(sqlite3.IntegrityError):
            self.publish()
        self.assertEqual(
            self.db.execute('SELECT COUNT(*) FROM editions').fetchone()[0], 1
        )
        self.assertEqual(
            self.db.execute('SELECT COUNT(*) FROM revisions').fetchone()[0], 1
        )

    def test_failed_publication_rolls_back_and_preserves_previous(self):
        self.lease()
        self.publish()
        with self.assertRaises(sqlite3.OperationalError):
            self.publish('2026-10-24', broken=True)
        self.assertEqual(
            self.db.execute('SELECT MAX(edition_date) FROM editions').fetchone()[0],
            '2026-10-23',
        )
        self.assertEqual(
            self.db.execute('SELECT COUNT(*) FROM revisions').fetchone()[0], 1
        )

    def test_lost_lease_cannot_publish_even_if_model_finished(self):
        self.lease()
        self.db.commit()
        with self.assertRaises(sqlite3.IntegrityError):
            self.publish(owner='other-worker')
        self.assertEqual(
            self.db.execute('SELECT COUNT(*) FROM editions').fetchone()[0], 0
        )

    def test_database_clock_enforces_deadline_and_records_publication_time(self):
        self.lease()
        self.publish()
        self.assertEqual(
            self.db.execute('SELECT published_at FROM editions').fetchone()[0], 103
        )
        self.assertEqual(
            json.loads(self.db.execute('SELECT payload FROM revisions').fetchone()[0])[
                'published_at'
            ],
            103,
        )
        self.lease('run-a', 3600)
        self.clock = 3700
        with self.assertRaises(sqlite3.IntegrityError):
            self.publish('2026-10-24')
        self.assertEqual(
            self.db.execute('SELECT COUNT(*) FROM editions').fetchone()[0], 1
        )

    def test_correction_preserves_earlier_revision_and_checks_expected_version(self):
        self.lease()
        self.publish()
        with self.db:
            self.db.execute(
                self.sql('correct_revision.sql'),
                (
                    '2026-10-23',
                    2,
                    150,
                    'Correct affected version',
                    '{"corrected":true}',
                    '2026-10-23',
                    1,
                ),
            )
            self.db.execute(
                self.sql('correct_pointer.sql'), (2, '2026-10-23', 1, '2026-10-23', 2)
            )
        original = self.db.execute(
            'SELECT payload FROM revisions WHERE revision=1'
        ).fetchone()[0]
        self.assertEqual(json.loads(original)['date'], '2026-10-23')
        self.assertEqual(
            self.db.execute('SELECT current_revision FROM editions').fetchone()[0], 2
        )
        with self.db:
            self.db.execute(
                self.sql('correct_revision.sql'),
                ('2026-10-23', 3, 160, 'Stale editor', '{}', '2026-10-23', 1),
            )
            self.db.execute(
                self.sql('correct_pointer.sql'), (3, '2026-10-23', 1, '2026-10-23', 3)
            )
        self.assertEqual(
            self.db.execute('SELECT COUNT(*) FROM revisions').fetchone()[0], 2
        )
        self.assertEqual(
            self.db.execute('SELECT current_revision FROM editions').fetchone()[0], 2
        )

    def document(self, identifier, url, retrieved, first_seen=10, source='origin'):
        self.db.execute(
            'INSERT INTO documents VALUES(?,?,?,?,?,?,?,?,?,?)',
            (
                identifier,
                url,
                source,
                first_seen,
                retrieved,
                None,
                None,
                identifier,
                identifier,
                json.dumps({'id': identifier}),
            ),
        )

    def test_cutoff_snapshot_retains_original_version_and_bounds_sources(self):
        self.document('before', 'https://example.com/a', 20)
        self.document('after', 'https://example.com/a', 101)
        self.document('unseen', 'https://example.com/b', 20, first_seen=101)
        self.document('other', 'https://other.example.com/c', 10, source='other')
        for i in range(15):
            self.document(
                f'bounded-{i}',
                f'https://bounded.example.com/{i}',
                30 + i,
                source='bounded',
            )
        rows = self.db.execute(self.sql('candidates.sql'), (100, 100, 0)).fetchall()
        ids = [json.loads(row[0])['id'] for row in rows]
        self.assertIn('before', ids)
        self.assertIn('other', ids)
        self.assertNotIn('after', ids)
        self.assertNotIn('unseen', ids)
        self.assertEqual(sum(item.startswith('bounded-') for item in ids), 6)

    def test_community_discovery_gets_more_room_within_total_budget(self):
        for source in ['hacker-news', 'x-search'] + [f'feed-{i}' for i in range(20)]:
            for i in range(15):
                self.document(
                    f'{source}-{i}',
                    f'https://example.com/{source}/{i}',
                    30 + i,
                    source=source,
                )
        rows = self.db.execute(self.sql('candidates.sql'), (100, 100, 0)).fetchall()
        ids = [json.loads(row[0])['id'] for row in rows]
        self.assertEqual(len(ids), 120)
        for source, limit in [('hacker-news', 10), ('x-search', 8)]:
            selected = {item for item in ids if item.startswith(f'{source}-')}
            self.assertEqual(selected, {f'{source}-{i}' for i in range(15 - limit, 15)})
        for i in range(20):
            count = sum(item.startswith(f'feed-{i}-') for item in ids)
            self.assertGreater(count, 0)
            self.assertLessEqual(count, 6)


if __name__ == '__main__':
    unittest.main()
