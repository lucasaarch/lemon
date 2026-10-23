"""Offline checks for the release publication gate."""
import unittest

from release_notes import release_notes

VERSION = '1.0.0'


class ReleaseNotesTests(unittest.TestCase):
    def test_extracts_only_the_matching_release(self):
        changelog = '# Changes\n## [1.0.0] - 2026-10-23\n\nNew API.\n\n## [0.1.0]\nOld API.\n'
        self.assertEqual(release_notes('v1.0.0', VERSION, changelog), 'New API.')

    def test_rejects_wrong_tag_missing_notes_and_empty_notes(self):
        for tag, changelog in [('v1.1.0', '## [1.0.0]\nNotes'),
                               ('v1.0.0', '## [0.1.0]\nOld'),
                               ('v1.0.0', '## [1.0.0]\n\n')]:
            with self.subTest(tag=tag, changelog=changelog):
                with self.assertRaises(ValueError):
                    release_notes(tag, VERSION, changelog)
