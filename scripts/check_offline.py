"""Run real checks that require no downloaded Rust crates or npm packages."""

import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
(ROOT / '.cache').mkdir(exist_ok=True)
commands = [
    [
        'rustc',
        '--edition=2021',
        '--test',
        'worker/src/rules.rs',
        '-o',
        '.cache/rules-tests',
    ],
    ['.cache/rules-tests'],
    ['cargo', 'fmt', '--all', '--check'],
    [sys.executable, '-m', 'unittest', 'discover', '-s', 'tests', '-v'],
    ['node', '--experimental-strip-types', '--test', 'tests/api.test.ts'],
]
for command in commands:
    print('Running:', ' '.join(command), flush=True)
    subprocess.run(command, cwd=ROOT, check=True)
