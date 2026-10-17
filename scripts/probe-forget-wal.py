#!/usr/bin/env python3
"""Reproduce WAL cleanup semantics with synthetic, disposable SQLite data.

This is not a Rust, rusqlite, SQLCipher, encryption, or platform gate. It uses
Python's standard-library SQLite and never opens a Soul/user database.
"""
from __future__ import annotations

import pathlib
import sqlite3
import tempfile
import unittest


def confirmed_truncate(row: tuple[int, int, int]) -> bool:
    return row == (0, 0, 0)


class WalSemantics(unittest.TestCase):
    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory(prefix="soul-wal-probe-")
        self.addCleanup(self.directory.cleanup)
        self.path = pathlib.Path(self.directory.name) / "synthetic.db"
        self.writer = sqlite3.connect(self.path, isolation_level=None, timeout=0)
        self.addCleanup(self.writer.close)
        self.assertEqual(self.writer.execute("PRAGMA journal_mode=WAL").fetchone(), ("wal",))
        self.writer.execute("PRAGMA wal_autocheckpoint=0")
        self.writer.execute("PRAGMA secure_delete=ON")
        self.writer.execute("CREATE TABLE synthetic_keys (id INTEGER PRIMARY KEY, value TEXT)")
        self.writer.execute("INSERT INTO synthetic_keys VALUES (1, 'synthetic-key-not-a-secret')")
        self.reader = sqlite3.connect(self.path, isolation_level=None, timeout=0)
        self.addCleanup(self.reader.close)

    def checkpoint(self) -> tuple[int, int, int]:
        row = self.writer.execute("PRAGMA wal_checkpoint(TRUNCATE)").fetchone()
        self.assertIsNotNone(row)
        self.assertEqual(len(row), 3)
        return row

    def pin_reader(self) -> None:
        self.reader.execute("BEGIN")
        self.assertEqual(self.reader.execute("SELECT count(*) FROM synthetic_keys").fetchone(), (1,))

    def test_success_has_three_zeroes_and_empty_wal(self) -> None:
        row = self.checkpoint()
        self.assertEqual(row, (0, 0, 0))
        self.assertTrue(confirmed_truncate(row))
        self.assertEqual(pathlib.Path(str(self.path) + "-wal").stat().st_size, 0)

    def test_busy_is_a_successful_query_but_not_cleanup(self) -> None:
        self.pin_reader()
        self.writer.execute("DELETE FROM synthetic_keys WHERE id=1")
        row = self.checkpoint()
        self.assertEqual(row[0], 1)
        self.assertFalse(confirmed_truncate(row))
        self.assertGreater(pathlib.Path(str(self.path) + "-wal").stat().st_size, 0)
        self.assertEqual(self.writer.execute("SELECT count(*) FROM synthetic_keys").fetchone(), (0,))
        self.assertEqual(self.reader.execute("SELECT count(*) FROM synthetic_keys").fetchone(), (1,))
        print("observed busy tuple:", row)

    def test_all_frames_copied_still_does_not_mean_truncated(self) -> None:
        self.pin_reader()
        row = self.checkpoint()
        self.assertEqual(row[0], 1)
        self.assertEqual(row[1], row[2])
        self.assertGreater(row[1], 0)
        self.assertFalse(confirmed_truncate(row))
        self.assertGreater(pathlib.Path(str(self.path) + "-wal").stat().st_size, 0)
        print("observed copied-but-not-truncated tuple:", row)

    def test_retry_cleanup_does_not_repeat_the_destructive_statement(self) -> None:
        self.pin_reader()
        self.writer.execute("BEGIN IMMEDIATE")
        self.writer.execute("DELETE FROM synthetic_keys WHERE id=1")
        self.writer.execute("COMMIT")
        self.assertFalse(confirmed_truncate(self.checkpoint()))
        self.reader.execute("ROLLBACK")
        changes = self.writer.total_changes
        statements: list[str] = []
        self.writer.set_trace_callback(statements.append)
        self.assertTrue(confirmed_truncate(self.checkpoint()))
        self.assertTrue(confirmed_truncate(self.checkpoint()))
        self.writer.set_trace_callback(None)
        self.assertEqual(statements, ["PRAGMA wal_checkpoint(TRUNCATE)"] * 2)
        self.assertEqual(self.writer.total_changes, changes)
        self.assertEqual(self.writer.execute("SELECT count(*) FROM synthetic_keys").fetchone(), (0,))
        self.writer.close()
        reopened = sqlite3.connect(self.path)
        self.addCleanup(reopened.close)
        self.assertEqual(reopened.execute("SELECT count(*) FROM synthetic_keys").fetchone(), (0,))

    def test_failure_before_commit_rolls_back_all_destruction(self) -> None:
        self.writer.execute("BEGIN IMMEDIATE")
        self.writer.execute("DELETE FROM synthetic_keys WHERE id=1")
        with self.assertRaises(sqlite3.OperationalError):
            self.writer.execute("INSERT INTO nonexistent_table VALUES (1)")
        self.writer.execute("ROLLBACK")
        self.assertEqual(self.writer.execute("SELECT count(*) FROM synthetic_keys").fetchone(), (1,))

    def test_non_wal_is_not_confirmed_wal_cleanup(self) -> None:
        with sqlite3.connect(":memory:") as database:
            row = database.execute("PRAGMA wal_checkpoint(TRUNCATE)").fetchone()
            self.assertEqual(row, (0, -1, -1))
            self.assertFalse(confirmed_truncate(row))


if __name__ == "__main__":
    print("Python SQLite", sqlite3.sqlite_version, "- synthetic semantics probe, NOT a Soul gate")
    unittest.main(verbosity=2)
