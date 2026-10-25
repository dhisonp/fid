import json
import subprocess
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / 'scripts/publication_state.py'


class PublicationStateTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.remote = self.root / 'remote.git'
        self.repo = self.root / 'application'
        self.command('git', 'init', '--bare', str(self.remote))
        self.command('git', 'init', '-b', 'main', str(self.repo))
        self.command('git', 'config', 'user.name', 'Test', cwd=self.repo)
        self.command('git', 'config', 'user.email', 'test@example.com', cwd=self.repo)
        (self.repo / 'app.txt').write_text('Application source')
        self.command('git', 'add', '.', cwd=self.repo)
        self.command('git', 'commit', '-m', 'Create application', cwd=self.repo)
        self.command('git', 'remote', 'add', 'origin', str(self.remote), cwd=self.repo)
        self.command('git', 'push', 'origin', 'main', cwd=self.repo)

    def command(self, *args, cwd=None, check=True):
        return subprocess.run(
            args, cwd=cwd, check=check, capture_output=True, text=True
        )

    def state(self, command, directory):
        return self.command('python3', str(SCRIPT), command, directory, cwd=self.repo)

    def test_first_publication_and_fresh_checkout_preserve_history_only(self):
        self.state('prepare', '.fid')
        state = self.repo / '.fid'
        self.assertFalse((state / 'app.txt').exists())
        (state / 'state.json').write_text(json.dumps({'version': 1}))
        (state / 'editions').mkdir()
        (state / 'editions' / 'saved.json').write_text('{"published":true}')
        (state / 'work').mkdir()
        (state / 'work' / 'private-run.json').write_text('not published')
        self.state('save', '.fid')
        files = self.command('git', 'ls-tree', '-r', '--name-only', 'HEAD', cwd=state)
        self.assertEqual(
            files.stdout.splitlines(), ['editions/saved.json', 'state.json']
        )
        self.state('prepare', '.restored')
        self.assertEqual(
            (self.repo / '.restored/editions/saved.json').read_text(),
            '{"published":true}',
        )
        self.state('save', '.restored')
        self.assertEqual((self.repo / 'app.txt').read_text(), 'Application source')

    def test_existing_directory_and_remote_errors_do_not_reset_state(self):
        existing = self.repo / '.fid'
        existing.mkdir()
        (existing / 'marker').write_text('keep')
        result = self.command(
            'python3', str(SCRIPT), 'prepare', '.fid', cwd=self.repo, check=False
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual((existing / 'marker').read_text(), 'keep')
        self.command(
            'git', 'remote', 'set-url', 'origin', '/missing/fid.git', cwd=self.repo
        )
        result = self.command(
            'python3', str(SCRIPT), 'prepare', '.fresh', cwd=self.repo, check=False
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse((self.repo / '.fresh').exists())


if __name__ == '__main__':
    unittest.main()
