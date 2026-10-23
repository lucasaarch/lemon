"""Validate a release tag against Cargo.toml and extract its changelog section."""
import json
import re
import subprocess
import sys
from pathlib import Path


def release_notes(tag, version, changelog):
    """Return release notes, or fail before publication on a mismatched version."""
    if tag != f'v{version}':
        raise ValueError(f'tag {tag!r} does not match package version {version!r}')
    section = re.search(
        rf'^## \[{re.escape(version)}\][^\n]*\n(.*?)(?=^## |\Z)',
        changelog, re.MULTILINE | re.DOTALL,
    )
    if section is None or not section[1].strip():
        raise ValueError(f'CHANGELOG.md needs release notes for {version}')
    return section[1].strip()


def main():
    """Read the repository files and print notes for the supplied exact tag."""
    if len(sys.argv) != 2:
        raise SystemExit('usage: python3 scripts/release_notes.py v1.0.0')
    root = Path(__file__).resolve().parent.parent
    try:
        metadata = json.loads(subprocess.check_output(
            ['cargo', 'metadata', '--locked', '--no-deps', '--format-version', '1'],
            cwd=root, text=True,
        ))
        package = next(package for package in metadata['packages']
                       if Path(package['manifest_path']).resolve() == root / 'Cargo.toml')
        notes = release_notes(sys.argv[1], package['version'],
                              (root / 'CHANGELOG.md').read_text())
    except ValueError as error:
        raise SystemExit(str(error)) from error
    print(notes)


if __name__ == '__main__':
    main()
