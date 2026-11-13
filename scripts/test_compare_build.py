"""Offline regression tests; generated fixtures are never Mainnet evidence."""
import hashlib
import json
import pathlib
import subprocess
import sys
import tempfile
import unittest

SCRIPT = pathlib.Path(__file__).with_name("compare-build.py")


class BuildComparisonTests(unittest.TestCase):
    def compare(self, binary, expected, commit="a" * 40):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            (root / "program.so").write_bytes(binary)
            (root / "snapshot.json").write_text(json.dumps({
                "normalizedProgramSha256": hashlib.sha256(expected.rstrip(b"\0")).hexdigest(),
                "programId": "offline-test-only",
                "observedAtUtc": "2026-01-01T00:00:00Z",
                "contextSlot": 1,
                "deploymentSlot": "1",
            }))
            result = subprocess.run([
                sys.executable, str(SCRIPT), str(root / "program.so"),
                str(root / "snapshot.json"), commit, str(root / "result.json"),
            ], capture_output=True, text=True)
            output = root / "result.json"
            return result, json.loads(output.read_text()) if output.exists() else None

    def test_matching_bytes(self):
        result, report = self.compare(b"\x7fELFexample", b"\x7fELFexample")
        self.assertEqual(result.returncode, 0)
        self.assertTrue(report["matchesSnapshot"])

    def test_program_padding_is_ignored(self):
        result, report = self.compare(b"\x7fELFexample\0\0", b"\x7fELFexample")
        self.assertEqual(result.returncode, 0)
        self.assertTrue(report["matchesSnapshot"])

    def test_one_changed_byte_fails(self):
        result, report = self.compare(b"\x7fELFexamplf", b"\x7fELFexample")
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(report["matchesSnapshot"])

    def test_internal_zero_is_not_ignored(self):
        result, report = self.compare(b"\x7fELFex\0ample", b"\x7fELFexample")
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(report["matchesSnapshot"])

    def test_invalid_source_commit_fails(self):
        result, report = self.compare(b"test", b"test", "main")
        self.assertNotEqual(result.returncode, 0)
        self.assertIsNone(report)


if __name__ == "__main__":
    unittest.main()
