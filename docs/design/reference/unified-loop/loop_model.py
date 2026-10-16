"""Synthetic, local reference model. This is not Soul production code.

SQLite transactions and reopen recovery are real; all data, analysis, remote
results and notification acknowledgements are simulated. No network is used.
"""
from contextlib import contextmanager
import fcntl
from pathlib import Path
import sqlite3
from uuid import uuid4


class Refused(Exception):
    pass


class InjectedCrash(Exception):
    """An injected interruption; not a physical power-loss simulation."""


def uid():
    return str(uuid4())


SCHEMA = """
CREATE TABLE IF NOT EXISTS sources (
 id TEXT PRIMARY KEY, revision INTEGER NOT NULL, body TEXT, forgotten INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS goals (
 id TEXT PRIMARY KEY, revision INTEGER NOT NULL, detail TEXT NOT NULL, active INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS purposes (
 id TEXT PRIMARY KEY, revision INTEGER NOT NULL, allowed INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS cases (
 id TEXT PRIMARY KEY, source_id TEXT NOT NULL REFERENCES sources(id),
 goal_id TEXT NOT NULL REFERENCES goals(id), purpose_id TEXT NOT NULL REFERENCES purposes(id),
 handling TEXT NOT NULL DEFAULT 'open', snooze_until INTEGER,
 muted INTEGER NOT NULL DEFAULT 0, UNIQUE(source_id, goal_id, purpose_id));
CREATE TABLE IF NOT EXISTS items (
 id TEXT PRIMARY KEY, case_id TEXT NOT NULL REFERENCES cases(id), generation INTEGER NOT NULL,
 source_revision INTEGER NOT NULL, goal_revision INTEGER NOT NULL,
 purpose_revision INTEGER NOT NULL, validity TEXT NOT NULL, result TEXT,
 UNIQUE(case_id, generation));
CREATE TABLE IF NOT EXISTS work (
 id TEXT PRIMARY KEY, item_id TEXT NOT NULL UNIQUE REFERENCES items(id), state TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS deliveries (
 id TEXT PRIMARY KEY, item_id TEXT NOT NULL REFERENCES items(id), channel TEXT NOT NULL,
 state TEXT NOT NULL, opaque_target TEXT NOT NULL, generic_text TEXT NOT NULL,
 UNIQUE(item_id, channel));
CREATE TABLE IF NOT EXISTS executions (
 id TEXT PRIMARY KEY, item_id TEXT NOT NULL REFERENCES items(id), origin TEXT NOT NULL,
 body TEXT, approval_id TEXT UNIQUE, consumed INTEGER NOT NULL DEFAULT 0,
 state TEXT NOT NULL, result TEXT, accepted INTEGER NOT NULL DEFAULT 0);
CREATE TABLE IF NOT EXISTS feedback (
 id TEXT PRIMARY KEY, item_id TEXT NOT NULL REFERENCES items(id), kind TEXT NOT NULL);
"""


class Loop:
    GENERIC_NOTICE = "Soul 有一项待查看的辅助内容。"

    def __init__(self, path):
        self.path = str(Path(path).expanduser().resolve())
        # A new instance must not recover a live owner's work. Linux-only
        # reference lease; this does not replace Soul's existing OS lease.
        self.lease = None
        self.db = None
        self.closed = True
        self._recovered = False
        try:
            self.lease = open(self.path + ".lease", "a+b")
            try:
                fcntl.flock(self.lease, fcntl.LOCK_EX | fcntl.LOCK_NB)
            except BlockingIOError as error:
                raise Refused("live_owner") from error
            self.db = sqlite3.connect(self.path, isolation_level=None)
            self.db.row_factory = sqlite3.Row
            self.db.execute("PRAGMA foreign_keys=ON")
            self.db.execute("PRAGMA synchronous=FULL")
            self.db.executescript(SCHEMA)
            self.recover()
            self.closed = False
        except BaseException:
            if self.db is not None:
                self.db.close()
            if self.lease is not None:
                self.lease.close()
            raise

    def close(self):
        if not self.closed:
            self.db.close()
            self.lease.close()
            self.closed = True

    @contextmanager
    def transaction(self):
        self.db.execute("BEGIN IMMEDIATE")
        try:
            yield
        except BaseException:
            self.db.execute("ROLLBACK")
            raise
        else:
            self.db.execute("COMMIT")

    def one(self, query, args=()):
        row = self.db.execute(query, args).fetchone()
        if row is None:
            raise Refused("missing")
        return row

    @staticmethod
    def checkpoint(requested, at):
        if requested == at:
            raise InjectedCrash(at)

    def set_goal(self, goal_id, detail, active=True):
        with self.transaction():
            self.db.execute(
                "INSERT INTO goals VALUES (?,1,?,?) ON CONFLICT(id) DO UPDATE SET "
                "revision=revision+1, detail=excluded.detail, active=excluded.active",
                (goal_id, detail, int(active)),
            )
            self.invalidate("goal_id", goal_id)

    def set_purpose(self, purpose_id, allowed):
        with self.transaction():
            self.db.execute(
                "INSERT INTO purposes VALUES (?,1,?) ON CONFLICT(id) DO UPDATE SET "
                "revision=revision+1, allowed=excluded.allowed", (purpose_id, int(allowed)),
            )
            self.invalidate("purpose_id", purpose_id)

    def invalidate(self, dimension, identifier, forgotten=False):
        # Called only inside a transaction. Four-axis history is preserved:
        # invalid content does not rewrite an unknown send as an unsent send.
        if dimension not in {"source_id", "goal_id", "purpose_id", "id"}:
            raise ValueError(dimension)
        case_ids = [r[0] for r in self.db.execute(
            f"SELECT id FROM cases WHERE {dimension}=?", (identifier,))]
        for case_id in case_ids:
            items = [r[0] for r in self.db.execute(
                "SELECT id FROM items WHERE case_id=?", (case_id,))]
            for item_id in items:
                self.db.execute("UPDATE items SET validity=CASE WHEN validity='forgotten' "
                                "THEN 'forgotten' ELSE ? END, result=NULL WHERE id=?",
                                ("forgotten" if forgotten else "stale", item_id))
                self.db.execute("UPDATE work SET state='invalidated' WHERE item_id=? "
                                "AND state IN ('queued','running')", (item_id,))
                self.db.execute("UPDATE deliveries SET state='invalidated' WHERE item_id=? "
                                "AND state='queued'", (item_id,))
                self.db.execute("UPDATE executions SET body=NULL, result=NULL "
                                "WHERE item_id=?", (item_id,))

    def source_snapshot(self, case_id):
        case = self.one("SELECT * FROM cases WHERE id=?", (case_id,))
        source = self.one("SELECT * FROM sources WHERE id=?", (case["source_id"],))
        goal = self.one("SELECT * FROM goals WHERE id=?", (case["goal_id"],))
        purpose = self.one("SELECT * FROM purposes WHERE id=?", (case["purpose_id"],))
        if source["forgotten"] or not goal["active"] or not purpose["allowed"]:
            raise Refused("source_or_purpose_unavailable")
        return case, source, goal, purpose

    def valid(self, item_id, ready=False):
        item = self.one("SELECT * FROM items WHERE id=?", (item_id,))
        if item["validity"] not in ({"current"} if ready else {"pending", "current"}):
            raise Refused("stale_item")
        case, source, goal, purpose = self.source_snapshot(item["case_id"])
        latest = self.one("SELECT MAX(generation) AS generation FROM items WHERE case_id=?",
                          (case["id"],))["generation"]
        if (item["generation"] != latest or
                item["source_revision"] != source["revision"] or
                item["goal_revision"] != goal["revision"] or
                item["purpose_revision"] != purpose["revision"]):
            raise Refused("changed_dependency")
        return item

    def new_item(self, case_id, fault=None):
        _, source, goal, purpose = self.source_snapshot(case_id)
        generation = self.one("SELECT COALESCE(MAX(generation),0)+1 AS n FROM items "
                              "WHERE case_id=?", (case_id,))["n"]
        self.invalidate("id", case_id)
        item_id = uid()
        self.db.execute("INSERT INTO items VALUES (?,?,?,?,?,?,'pending',NULL)",
                        (item_id, case_id, generation, source["revision"],
                         goal["revision"], purpose["revision"]))
        self.checkpoint(fault, "after_item")
        self.db.execute("INSERT INTO work VALUES (?,?,'queued')", (uid(), item_id))
        self.checkpoint(fault, "after_work")
        return item_id

    def ingest(self, source_id, body, goal_id, purpose_id, fault=None):
        """One synthetic source record, one explicit goal/purpose association.

        Same source ID + unchanged body is a duplicate. Changed body is an
        explicit source revision, not a guessed message-level merge rule.
        """
        with self.transaction():
            old = self.db.execute("SELECT * FROM sources WHERE id=?", (source_id,)).fetchone()
            if old and old["forgotten"]:
                raise Refused("forgotten_source_cannot_reappear")
            if not old:
                self.db.execute("INSERT INTO sources VALUES (?,1,?,0)", (source_id, body))
            elif old["body"] != body:
                self.db.execute("UPDATE sources SET revision=revision+1, body=? WHERE id=?",
                                (body, source_id))
                self.invalidate("source_id", source_id)
            self.checkpoint(fault, "after_source")
            case = self.db.execute("SELECT * FROM cases WHERE source_id=? AND goal_id=? "
                                   "AND purpose_id=?", (source_id, goal_id, purpose_id)).fetchone()
            if case is None:
                case_id = uid()
                self.db.execute("INSERT INTO cases(id,source_id,goal_id,purpose_id) VALUES (?,?,?,?)",
                                (case_id, source_id, goal_id, purpose_id))
            else:
                case_id = case["id"]
                latest = self.db.execute("SELECT id FROM items WHERE case_id=? "
                                         "ORDER BY generation DESC LIMIT 1", (case_id,)).fetchone()
                if latest:
                    try:
                        self.valid(latest["id"])
                        return latest["id"]
                    except Refused:
                        pass
            return self.new_item(case_id, fault)

    def recompute(self, case_id):
        with self.transaction():
            return self.new_item(case_id)

    def change_source(self, source_id, body):
        with self.transaction():
            source = self.one("SELECT * FROM sources WHERE id=?", (source_id,))
            if source["forgotten"]:
                raise Refused("forgotten_source")
            self.db.execute("UPDATE sources SET revision=revision+1, body=? WHERE id=?",
                            (body, source_id))
            self.invalidate("source_id", source_id)

    def forget(self, source_id):
        with self.transaction():
            self.one("SELECT id FROM sources WHERE id=?", (source_id,))
            self.db.execute("UPDATE sources SET revision=revision+1, body=NULL, forgotten=1 "
                            "WHERE id=?", (source_id,))
            self.invalidate("source_id", source_id, forgotten=True)

    def claim(self, item_id):
        with self.transaction():
            self.valid(item_id)
            changed = self.db.execute("UPDATE work SET state='running' WHERE item_id=? "
                                      "AND state='queued'", (item_id,)).rowcount
            if changed != 1:
                raise Refused("work_not_queued")

    def publish(self, item_id, fault=None):
        with self.transaction():
            item = self.valid(item_id)
            work = self.one("SELECT * FROM work WHERE item_id=?", (item_id,))
            if work["state"] == "completed":
                return item["result"]
            if work["state"] != "running":
                raise Refused("work_not_running")
            case, source, goal, _ = self.source_snapshot(item["case_id"])
            result = "合成观察：" + source["body"] + "；当前目标：" + goal["detail"]
            self.db.execute("UPDATE items SET validity='current', result=? WHERE id=?",
                            (result, item_id))
            self.checkpoint(fault, "after_result")
            self.db.execute("INSERT INTO deliveries VALUES (?,?,'inbox','available',?,?)",
                            (uid(), item_id, case["id"], self.GENERIC_NOTICE))
            self.db.execute("UPDATE work SET state='completed' WHERE item_id=?", (item_id,))
            self.checkpoint(fault, "after_completion")
            return result

    def recover(self):
        if self._recovered:
            raise Refused("lease_already_recovered")
        with self.transaction():
            # A fresh exclusive lease proves the prior physical owner ended.
            for row in self.db.execute("SELECT item_id FROM work WHERE state='running'").fetchall():
                try:
                    self.valid(row[0])
                    state = "queued"
                except Refused:
                    state = "invalidated"
                self.db.execute("UPDATE work SET state=? WHERE item_id=?", (state, row[0]))
            self.db.execute("UPDATE deliveries SET state='unknown' WHERE state='inflight'")
            self.db.execute("UPDATE executions SET state='unknown' WHERE state='inflight'")
            # Approval is deliberately not resumed after a restart.
            self.db.execute("UPDATE executions SET state='interrupted', body=NULL "
                            "WHERE state IN ('prepared','approved')")
        self._recovered = True

    def current_case_action(self, case_id, expected_generation):
        case = self.one("SELECT * FROM cases WHERE id=?", (case_id,))
        latest = self.one("SELECT * FROM items WHERE case_id=? ORDER BY generation DESC LIMIT 1",
                          (case_id,))
        if latest["generation"] != expected_generation:
            raise Refused("stale_case_action")
        self.valid(latest["id"], ready=True)
        if case["handling"] in {"resolved", "dismissed"}:
            raise Refused("item_handled")
        return case

    def snooze(self, case_id, expected_generation, until):
        with self.transaction():
            self.current_case_action(case_id, expected_generation)
            self.db.execute("UPDATE cases SET handling='snoozed', snooze_until=? WHERE id=?",
                            (until, case_id))

    def mute(self, case_id, expected_generation, enabled=True):
        with self.transaction():
            self.current_case_action(case_id, expected_generation)
            self.db.execute("UPDATE cases SET muted=? WHERE id=?", (int(enabled), case_id))

    def notification_eligible(self, item_id, now):
        item = self.valid(item_id, ready=True)
        case = self.one("SELECT * FROM cases WHERE id=?", (item["case_id"],))
        return not (case["muted"] or case["handling"] in {"resolved", "dismissed"} or
                    (case["snooze_until"] is not None and now < case["snooze_until"]))

    def queue_notification(self, item_id, now):
        with self.transaction():
            if not self.notification_eligible(item_id, now):
                return None
            item = self.valid(item_id, ready=True)
            old = self.db.execute("SELECT id FROM deliveries WHERE item_id=? "
                                  "AND channel='notification'", (item_id,)).fetchone()
            if old:
                return old["id"]
            # Recompute alone is not a new notification reason. A different
            # item UUID must not turn an unknown send into an automatic retry.
            previous = self.db.execute(
                "SELECT d.id FROM deliveries d JOIN items i ON i.id=d.item_id "
                "WHERE i.case_id=? AND i.source_revision=? AND i.goal_revision=? "
                "AND i.purpose_revision=? AND d.channel='notification' "
                "AND d.state IN ('inflight','submitted','opened','unknown') LIMIT 1",
                (item["case_id"], item["source_revision"], item["goal_revision"],
                 item["purpose_revision"]),
            ).fetchone()
            if previous:
                return None
            delivery_id = uid()
            self.db.execute("INSERT INTO deliveries VALUES (?,?,'notification','queued',?,?)",
                            (delivery_id, item_id, item["case_id"], self.GENERIC_NOTICE))
            return delivery_id

    def dispatch_notification(self, delivery_id, now):
        with self.transaction():
            delivery = self.one("SELECT * FROM deliveries WHERE id=?", (delivery_id,))
            if delivery["channel"] != "notification" or delivery["state"] != "queued":
                raise Refused("notification_not_queued")
            if not self.notification_eligible(delivery["item_id"], now):
                raise Refused("notification_suppressed")
            self.db.execute("UPDATE deliveries SET state='inflight' WHERE id=?", (delivery_id,))
            return {"text": self.GENERIC_NOTICE, "opaque_target": delivery["opaque_target"]}

    def notification_result(self, delivery_id, outcome):
        if outcome not in {"submitted", "unknown"}:
            raise ValueError(outcome)
        with self.transaction():
            changed = self.db.execute("UPDATE deliveries SET state=? WHERE id=? AND state='inflight'",
                                      (outcome, delivery_id)).rowcount
            if changed != 1:
                raise Refused("notification_not_inflight")

    def open_notification(self, delivery_id):
        with self.transaction():
            delivery = self.one("SELECT * FROM deliveries WHERE id=?", (delivery_id,))
            if delivery["channel"] != "notification" or delivery["state"] not in {"submitted", "unknown", "opened"}:
                raise Refused("notification_not_openable")
            # A notification is navigation, never a version-bound action.
            # Its stable case pointer resolves only to the latest revision.
            case = self.one("SELECT * FROM cases WHERE id=?", (delivery["opaque_target"],))
            source = self.one("SELECT * FROM sources WHERE id=?", (case["source_id"],))
            if source["forgotten"]:
                raise Refused("forgotten_source")
            item = self.one("SELECT * FROM items WHERE case_id=? ORDER BY generation DESC LIMIT 1",
                            (case["id"],))
            view = {"case_id": case["id"], "item_id": item["id"], "generation": item["generation"]}
            try:
                self.valid(item["id"], ready=True)
                if case["handling"] in {"resolved", "dismissed"}:
                    view.update(status="handled", notice="这项辅助内容已经处理。")
                else:
                    view.update(status="current", result=item["result"])
            except Refused:
                view.update(status="unavailable", notice="依据已变化或不可用，请查看当前状态。")
            self.db.execute("UPDATE deliveries SET state='opened' WHERE id=?", (delivery_id,))
            return view

    def prepare_draft(self, item_id, origin):
        with self.transaction():
            item = self.valid(item_id, ready=True)
            case = self.one("SELECT * FROM cases WHERE id=?", (item["case_id"],))
            if case["handling"] in {"resolved", "dismissed"}:
                raise Refused("item_handled")
            preparation_id = uid()
            self.db.execute("INSERT INTO executions(id,item_id,origin,body,state) VALUES (?,?,?,?,'prepared')",
                            (preparation_id, item_id, origin, item["result"]))
            return preparation_id

    def approve(self, preparation_id):
        """Explicit user event; never called by analysis or notification paths."""
        with self.transaction():
            prepared = self.one("SELECT * FROM executions WHERE id=?", (preparation_id,))
            self.valid(prepared["item_id"], ready=True)
            if prepared["state"] != "prepared":
                raise Refused("not_prepared")
            approval_id = uid()
            self.db.execute("UPDATE executions SET state='approved', approval_id=? WHERE id=?",
                            (approval_id, preparation_id))
            return approval_id

    def dispatch_e1(self, preparation_id, approval_id):
        with self.transaction():
            prepared = self.one("SELECT * FROM executions WHERE id=?", (preparation_id,))
            item = self.valid(prepared["item_id"], ready=True)
            handling = self.one("SELECT handling FROM cases WHERE id=?", (item["case_id"],))[0]
            if handling in {"resolved", "dismissed"}:
                raise Refused("item_handled")
            if (prepared["state"] != "approved" or prepared["consumed"] or
                    approval_id != prepared["approval_id"] or not prepared["body"]):
                raise Refused("no_current_approval")
            self.db.execute("UPDATE executions SET state='inflight', consumed=1 WHERE id=?",
                            (preparation_id,))
            return {"origin": prepared["origin"], "body": prepared["body"]}

    def e1_result(self, preparation_id, outcome, text=None):
        if outcome not in {"succeeded", "unknown"}:
            raise ValueError(outcome)
        with self.transaction():
            prepared = self.one("SELECT * FROM executions WHERE id=?", (preparation_id,))
            if prepared["state"] != "inflight":
                raise Refused("not_inflight")
            accepted = False
            if outcome == "succeeded":
                try:
                    item = self.valid(prepared["item_id"], ready=True)
                    case = self.one("SELECT * FROM cases WHERE id=?", (item["case_id"],))
                    accepted = case["handling"] not in {"resolved", "dismissed"}
                except Refused:
                    pass
            self.db.execute("UPDATE executions SET state=?, result=?, accepted=?, body=NULL WHERE id=?",
                            (outcome, text if accepted else None, int(accepted), preparation_id))
            return accepted

    def give_feedback(self, item_id, kind, feedback_id):
        if kind not in {"copy", "done", "dismiss", "helpful", "unhelpful"}:
            raise ValueError(kind)
        with self.transaction():
            old = self.db.execute("SELECT * FROM feedback WHERE id=?", (feedback_id,)).fetchone()
            if old:
                if (old["item_id"], old["kind"]) != (item_id, kind):
                    raise Refused("feedback_identity_conflict")
                return
            item = self.valid(item_id, ready=True)
            self.db.execute("INSERT INTO feedback VALUES (?,?,?)", (feedback_id, item_id, kind))
            if kind in {"done", "dismiss"}:
                self.db.execute("UPDATE cases SET handling=? WHERE id=?",
                                ("resolved" if kind == "done" else "dismissed", item["case_id"]))
                self.db.execute("UPDATE deliveries SET state='cancelled' WHERE state='queued' "
                                "AND item_id IN (SELECT id FROM items WHERE case_id=?)", (item["case_id"],))
                self.db.execute("UPDATE executions SET state=CASE WHEN state IN ('prepared','approved') "
                                "THEN 'cancelled' ELSE state END, body=NULL, result=NULL "
                                "WHERE item_id IN (SELECT id FROM items WHERE case_id=?)", (item["case_id"],))

    def describe(self, item_id):
        item = dict(self.one("SELECT * FROM items WHERE id=?", (item_id,)))
        case = self.one("SELECT * FROM cases WHERE id=?", (item["case_id"],))
        try:
            self.valid(item_id, ready=True)
            readable = True
        except Refused:
            readable = False
        return {
            "item_id": item_id, "case_id": item["case_id"], "generation": item["generation"],
            "revisions": {k: item[k] for k in ("source_revision", "goal_revision", "purpose_revision")},
            "validity": item["validity"],
            "handling": case["handling"], "snooze_until": case["snooze_until"], "muted": bool(case["muted"]),
            "deliveries": [dict(r) for r in self.db.execute(
                "SELECT channel,state,opaque_target,generic_text FROM deliveries WHERE item_id=?", (item_id,))],
            "executions": [dict(r) for r in self.db.execute(
                "SELECT id,state,consumed,accepted FROM executions WHERE item_id=?", (item_id,))],
            "work": self.one("SELECT state FROM work WHERE item_id=?", (item_id,))[0],
            "result_available": readable and case["handling"] not in {"resolved", "dismissed"},
            "result": item["result"] if readable and case["handling"] not in {"resolved", "dismissed"} else None,
        }
