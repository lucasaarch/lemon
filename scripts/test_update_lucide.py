"""Offline regression tests for the release updater: python3 -m unittest discover -s scripts."""
import io
import json
import subprocess
import tarfile
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import update_lucide as updater

SVG = b'<svg viewBox="0 0 24 24"><path d="M2 2L22 22"/></svg>'


def archive(path, files):
    with tarfile.open(path, 'w:gz') as output:
        for name, content in files.items():
            member = tarfile.TarInfo(name)
            member.size = len(content)
            output.addfile(member, io.BytesIO(content))


class ReleaseUpdaterTests(unittest.TestCase):
    def test_relative_paths_keep_their_own_origin_when_shapes_are_combined(self):
        source = b'<svg viewBox="0 0 24 24"><path d="M18 6 6 18"/><path d=" m6 6 12 12"/></svg>'
        catalog = updater.generate_catalog([('x', source)], 'test')
        self.assertIn('M18 6 6 18 M0 0 m6 6 12 12', catalog)

    def test_fetches_latest_stable_release_from_official_endpoint(self):
        with patch.object(updater, 'urlopen', return_value=io.BytesIO(json.dumps({
            'tag_name': '1.61.0', 'prerelease': False, 'draft': False,
        }).encode())) as fetch:
            self.assertEqual(updater.latest_release(), '1.61.0')
        self.assertEqual(fetch.call_args.args[0].full_url,
                         'https://api.github.com/repos/lucide-icons/lucide/releases/latest')

    def test_rejects_unstable_or_missing_release_metadata(self):
        for release in [{'tag_name': 'next', 'prerelease': True},
                        {'tag_name': 'draft', 'draft': True}, {}]:
            with self.subTest(release=release), patch.object(updater, 'urlopen',
                    return_value=io.BytesIO(json.dumps(release).encode())), self.assertRaises(ValueError):
                updater.latest_release()

    def test_reads_only_icons_and_license_and_records_the_release(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'release.tar.gz'
            archive(path, {'lucide-tag/icons/folder.svg': SVG,
                           'lucide-tag/icons/folder.json': b'ignored',
                           'lucide-tag/LICENSE': b'upstream license'})
            icons, license_text = updater.read_archive(path)
            self.assertEqual(len(icons), 1)
            self.assertEqual(license_text, 'upstream license')
            rust = updater.generate_catalog(icons, '1.61.0')
            self.assertIn('upstream release `1.61.0`', rust)
            self.assertIn('pub const FOLDER: IconData', rust)

    def test_rejects_unsafe_archives_and_missing_license(self):
        for files in [{'../outside.svg': SVG}, {'root/icons/folder.svg': SVG}]:
            with self.subTest(files=list(files)), tempfile.TemporaryDirectory() as directory:
                path = Path(directory) / 'release.tar.gz'
                archive(path, files)
                with self.assertRaises(ValueError):
                    updater.read_archive(path)

    def test_reports_unreadable_archive_members(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'release.tar.gz'
            archive(path, {'root/icons/folder.svg': SVG, 'root/LICENSE': b'license'})
            with patch.object(tarfile.TarFile, 'extractfile', return_value=None), \
                    self.assertRaisesRegex(ValueError, 'Cannot read archive member'):
                updater.read_archive(path)

    def test_conversion_or_formatting_failure_preserves_both_outputs(self):
        for unsupported in [True, False]:
            with self.subTest(unsupported=unsupported), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                (root / 'src/icons').mkdir(parents=True)
                (root / 'licenses').mkdir()
                catalog = root / 'src/icons/lucide.rs'
                license_path = root / 'licenses/lucide-LICENSE'
                catalog.write_text('existing catalog')
                license_path.write_text('existing license')
                path = root / 'release.tar.gz'
                svg = b'<svg viewBox="0 0 24 24"><image/></svg>' if unsupported else SVG
                archive(path, {'root/icons/folder.svg': svg, 'root/LICENSE': b'new license'})
                with patch.object(updater.subprocess, 'run',
                        side_effect=subprocess.CalledProcessError(1, 'rustfmt')), \
                        self.assertRaises((ValueError, subprocess.CalledProcessError)):
                    updater.update_catalog(root, path, '1.61.0')
                self.assertEqual(catalog.read_text(), 'existing catalog')
                self.assertEqual(license_path.read_text(), 'existing license')


if __name__ == '__main__':
    unittest.main()
