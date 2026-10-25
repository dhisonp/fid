"""Keep published editions on a durable Git branch, outside the application checkout."""

import argparse
import subprocess
from pathlib import Path

BRANCH = 'fid-data'


def git(*args, cwd=None, check=True):
    return subprocess.run(
        ['git', *args], cwd=cwd, check=check, text=True, capture_output=True
    )


def prepare(directory):
    if directory.exists():
        raise RuntimeError(f'{directory} already exists; refusing to replace state')
    remote = git('ls-remote', '--exit-code', '--heads', 'origin', BRANCH, check=False)
    if remote.returncode == 0:
        git('fetch', '--depth=1', 'origin', f'refs/heads/{BRANCH}')
        git('worktree', 'add', '--detach', str(directory), 'FETCH_HEAD')
        if not (directory / 'state.json').is_file():
            raise RuntimeError('The publication branch has no state manifest')
    elif remote.returncode == 2:
        git('worktree', 'add', '--detach', str(directory), 'HEAD')
        git('switch', '--orphan', BRANCH, cwd=directory)
    else:
        raise RuntimeError(f'Cannot read publication branch: {remote.stderr.strip()}')


def save(directory):
    git('config', 'user.name', 'github-actions[bot]', cwd=directory)
    git(
        'config',
        'user.email',
        '41898282+github-actions[bot]@users.noreply.github.com',
        cwd=directory,
    )
    paths = ['state.json'] + [
        str(path.relative_to(directory))
        for name in ['editions', 'documents']
        for path in sorted((directory / name).rglob('*.json'))
    ]
    git('add', '--', *paths, cwd=directory)
    changes = git('diff', '--cached', '--quiet', check=False, cwd=directory)
    if changes.returncode == 1:
        git('commit', '-m', 'Save published editions', cwd=directory)
    elif changes.returncode != 0:
        raise RuntimeError(changes.stderr)
    git('push', 'origin', f'HEAD:refs/heads/{BRANCH}', cwd=directory)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command', choices=['prepare', 'save'])
    parser.add_argument('directory', type=Path)
    args = parser.parse_args()
    try:
        {'prepare': prepare, 'save': save}[args.command](args.directory)
    except subprocess.CalledProcessError as error:
        raise SystemExit(error.stderr.strip() or str(error)) from error


if __name__ == '__main__':
    main()
