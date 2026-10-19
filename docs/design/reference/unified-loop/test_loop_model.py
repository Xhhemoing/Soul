"""Executable synthetic invariants, with real SQLite transactions/reopens."""
import json
from pathlib import Path
import tempfile
import time
import unittest
from unittest.mock import patch
from loop_model import InjectedCrash, Loop, Refused, uid


class LoopTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.path = Path(self.tmp.name) / "synthetic.db"
        self.model = Loop(self.path)
        self.model.set_goal("goal", "准备一次有依据的沟通")
        self.model.set_purpose("assist", True)

    def tearDown(self):
        self.model.close()
        self.tmp.cleanup()

    def restart(self):
        self.model.close()
        self.model = Loop(self.path)

    def item(self, ready=True):
        item = self.model.ingest("source", "合成记录：下周讨论", "goal", "assist")
        if ready:
            self.model.claim(item)
            self.model.publish(item)
        return item

    def test_whole_loop_four_axes_are_independent(self):
        item = self.item()
        delivery = self.model.queue_notification(item, 100)
        payload = self.model.dispatch_notification(delivery, 100)
        self.assertEqual(set(payload), {"text", "opaque_target"})
        self.assertEqual(payload["opaque_target"], self.model.describe(item)["case_id"])
        self.assertNotIn("下周", payload["text"])
        self.model.notification_result(delivery, "submitted")
        self.assertIn("合成记录", self.model.open_notification(delivery)["result"])
        prep = self.model.prepare_draft(item, "https://synthetic.invalid")
        with self.assertRaises(Refused):
            self.model.dispatch_e1(prep, "made-up-approval")
        approval = self.model.approve(prep)
        self.model.dispatch_e1(prep, approval)
        self.assertTrue(self.model.e1_result(prep, "succeeded", "合成草稿"))
        self.model.give_feedback(item, "copy", uid())
        copied = self.model.describe(item)
        self.assertEqual(copied["handling"], "open")
        self.assertEqual(copied["validity"], "current")
        self.assertFalse(copied["muted"])
        self.model.give_feedback(item, "done", uid())
        done = self.model.describe(item)
        self.assertEqual(done["handling"], "resolved")
        self.assertEqual(done["validity"], "current")
        self.assertEqual(done["executions"][0]["state"], "succeeded")
        self.assertFalse(done["result_available"])

    def test_ingest_and_work_rollback_together_at_all_checkpoints(self):
        for checkpoint in ("after_source", "after_item", "after_work"):
            with self.subTest(checkpoint=checkpoint):
                with self.assertRaises(InjectedCrash):
                    self.model.ingest("source", "synthetic", "goal", "assist", checkpoint)
                self.restart()
                for table in ("sources", "cases", "items", "work"):
                    self.assertEqual(self.model.db.execute(f"SELECT COUNT(*) FROM {table}").fetchone()[0], 0)

    def test_publish_and_completion_rollback_together_then_recover(self):
        for checkpoint in ("after_result", "after_completion"):
            with self.subTest(checkpoint=checkpoint):
                item = self.model.ingest(checkpoint, "synthetic", "goal", "assist")
                self.model.claim(item)
                with self.assertRaises(InjectedCrash):
                    self.model.publish(item, checkpoint)
                self.restart()
                state = self.model.describe(item)
                self.assertEqual(state["validity"], "pending")
                self.assertEqual(state["work"], "queued")
                self.assertEqual(state["deliveries"], [])
                self.assertIsNone(state["result"])
                self.model.claim(item)
                self.model.publish(item)
                self.restart()
                self.assertEqual(self.model.describe(item)["work"], "completed")

    def test_duplicates_do_not_duplicate_visible_effects(self):
        item = self.item()
        self.assertEqual(self.model.ingest("source", "合成记录：下周讨论", "goal", "assist"), item)
        self.model.publish(item)
        notice = self.model.queue_notification(item, 0)
        self.assertEqual(self.model.queue_notification(item, 0), notice)
        self.assertEqual(len(self.model.describe(item)["deliveries"]), 2)
        with self.assertRaises(Refused):
            self.model.claim(item)

    def test_each_dependency_change_invalidates_all_outputs(self):
        changes = (
            lambda: self.model.change_source("source", "replacement of equal length"),
            lambda: self.model.set_goal("goal", "新的目标"),
            lambda: self.model.set_purpose("assist", False),
        )
        for change in changes:
            item = self.item()
            delivery = self.model.queue_notification(item, 0)
            prep = self.model.prepare_draft(item, "https://synthetic.invalid")
            approval = self.model.approve(prep)
            change()
            self.assertFalse(self.model.describe(item)["result_available"])
            with self.assertRaises(Refused):
                self.model.dispatch_notification(delivery, 0)
            with self.assertRaises(Refused):
                self.model.dispatch_e1(prep, approval)
        self.restart()
        self.assertFalse(self.model.describe(item)["result_available"])

    def test_permission_aba_does_not_restore_old_approval(self):
        item = self.item()
        prep = self.model.prepare_draft(item, "https://synthetic.invalid")
        approval = self.model.approve(prep)
        self.model.set_purpose("assist", False)
        self.model.set_purpose("assist", True)
        with self.assertRaises(Refused):
            self.model.dispatch_e1(prep, approval)
        new = self.model.recompute(self.model.describe(item)["case_id"])
        self.model.claim(new)
        self.model.publish(new)
        newprep = self.model.prepare_draft(new, "https://synthetic.invalid")
        with self.assertRaises(Refused):
            self.model.dispatch_e1(newprep, approval)

    def test_snooze_and_mute_survive_recompute_and_restart(self):
        item = self.item()
        case = self.model.describe(item)["case_id"]
        self.model.snooze(case, 1, 500)
        self.model.mute(case, 1)
        newer = self.model.recompute(case)
        self.model.claim(newer)
        self.model.publish(newer)
        self.restart()
        state = self.model.describe(newer)
        self.assertEqual(state["snooze_until"], 500)
        self.assertTrue(state["muted"])
        self.assertIsNone(self.model.queue_notification(newer, 1000))
        self.model.mute(case, 2, False)
        self.assertIsNone(self.model.queue_notification(newer, 100))
        self.assertIsNotNone(self.model.queue_notification(newer, 500))

    def test_snooze_is_rechecked_after_notification_was_queued(self):
        item = self.item()
        notice = self.model.queue_notification(item, 0)
        self.model.snooze(self.model.describe(item)["case_id"], 1, 50)
        with self.assertRaises(Refused):
            self.model.dispatch_notification(notice, 1)
        self.model.dispatch_notification(notice, 50)

    def test_old_notification_navigates_current_generation_without_transferring_old_actions(self):
        item = self.item()
        notice = self.model.queue_notification(item, 0)
        self.model.dispatch_notification(notice, 0)
        self.model.notification_result(notice, "submitted")
        prep = self.model.prepare_draft(item, "https://synthetic.invalid")
        approval = self.model.approve(prep)
        newer = self.model.recompute(self.model.describe(item)["case_id"])
        self.model.claim(newer)
        self.model.publish(newer)
        view = self.model.open_notification(notice)
        self.assertEqual(view["status"], "current")
        self.assertEqual(view["item_id"], newer)
        self.assertEqual(view["generation"], 2)
        self.assertIsNone(self.model.queue_notification(newer, 100))
        with self.assertRaises(Refused):
            self.model.give_feedback(item, "done", uid())
        with self.assertRaises(Refused):
            self.model.dispatch_e1(prep, approval)
        self.assertEqual(self.model.describe(newer)["handling"], "open")

    def test_notification_navigation_shows_unavailable_instead_of_old_content(self):
        item = self.item()
        notice = self.model.queue_notification(item, 0)
        self.model.dispatch_notification(notice, 0)
        self.model.notification_result(notice, "submitted")
        self.model.change_source("source", "new fact")
        unavailable = self.model.open_notification(notice)
        self.assertEqual(unavailable["status"], "unavailable")
        self.assertNotIn("result", unavailable)
        newer = self.model.recompute(self.model.describe(item)["case_id"])
        self.model.claim(newer)
        self.model.publish(newer)
        current = self.model.open_notification(notice)
        self.assertEqual(current["item_id"], newer)
        self.assertIn("new fact", current["result"])

    def test_snooze_and_mute_require_current_generation(self):
        item = self.item()
        case = self.model.describe(item)["case_id"]
        newer = self.model.recompute(case)
        self.model.claim(newer)
        self.model.publish(newer)
        with self.assertRaisesRegex(Refused, "stale_case_action"):
            self.model.snooze(case, 1, 0)
        with self.assertRaisesRegex(Refused, "stale_case_action"):
            self.model.mute(case, 1)
        state = self.model.describe(newer)
        self.assertEqual(state["handling"], "open")
        self.assertFalse(state["muted"])
        self.assertIsNone(state["snooze_until"])

    def test_terminal_feedback_cancels_pending_work_without_reopening_or_rewriting_attempts(self):
        for feedback in ("done", "dismiss"):
            with self.subTest(feedback=feedback):
                item = self.model.ingest(feedback, "synthetic", "goal", "assist")
                self.model.claim(item)
                self.model.publish(item)
                case = self.model.describe(item)["case_id"]
                notice = self.model.queue_notification(item, 0)
                prepared = self.model.prepare_draft(item, "https://synthetic.invalid")
                approved = self.model.prepare_draft(item, "https://synthetic.invalid")
                approval = self.model.approve(approved)
                inflight = self.model.prepare_draft(item, "https://synthetic.invalid")
                self.model.dispatch_e1(inflight, self.model.approve(inflight))
                unknown = self.model.prepare_draft(item, "https://synthetic.invalid")
                self.model.dispatch_e1(unknown, self.model.approve(unknown))
                self.model.e1_result(unknown, "unknown")
                self.model.give_feedback(item, feedback, uid())
                with self.assertRaisesRegex(Refused, "item_handled"):
                    self.model.snooze(case, 1, 0)
                with self.assertRaisesRegex(Refused, "item_handled"):
                    self.model.mute(case, 1)
                with self.assertRaises(Refused):
                    self.model.dispatch_e1(approved, approval)
                with self.assertRaises(Refused):
                    self.model.dispatch_notification(notice, 0)
                states = {r["id"]: r for r in self.model.db.execute(
                    "SELECT * FROM executions WHERE item_id=?", (item,))}
                self.assertEqual(states[prepared]["state"], "cancelled")
                self.assertEqual(states[approved]["state"], "cancelled")
                self.assertEqual(states[inflight]["state"], "inflight")
                self.assertEqual(states[unknown]["state"], "unknown")
                self.assertTrue(all(r["body"] is None and r["result"] is None for r in states.values()))
                self.assertEqual(self.model.one("SELECT state FROM deliveries WHERE id=?", (notice,))[0], "cancelled")
                self.assertFalse(self.model.e1_result(inflight, "succeeded", "late body"))

    def test_unknown_notification_is_not_retried_after_reopen(self):
        item = self.item()
        notice = self.model.queue_notification(item, 0)
        self.model.dispatch_notification(notice, 0)
        self.restart()
        self.assertEqual(self.model.queue_notification(item, 0), notice)
        with self.assertRaises(Refused):
            self.model.dispatch_notification(notice, 0)
        self.assertEqual(self.model.one("SELECT state FROM deliveries WHERE id=?", (notice,))[0], "unknown")
        self.assertEqual(self.model.describe(item)["handling"], "open")

    def test_unknown_e1_cannot_retry_or_reuse_approval(self):
        item = self.item()
        prep = self.model.prepare_draft(item, "https://synthetic.invalid")
        approval = self.model.approve(prep)
        self.model.dispatch_e1(prep, approval)
        self.restart()
        self.assertEqual(self.model.describe(item)["executions"][0]["state"], "unknown")
        with self.assertRaises(Refused):
            self.model.dispatch_e1(prep, approval)
        with self.assertRaises(Refused):
            self.model.approve(prep)
        self.assertEqual(self.model.describe(item)["executions"][0]["consumed"], 1)

    def test_recompute_does_not_disguise_unknown_notification_as_new_send(self):
        item = self.item()
        notice = self.model.queue_notification(item, 0)
        self.model.dispatch_notification(notice, 0)
        self.model.notification_result(notice, "unknown")
        newer = self.model.recompute(self.model.describe(item)["case_id"])
        self.model.claim(newer)
        self.model.publish(newer)
        self.restart()
        self.assertIsNone(self.model.queue_notification(newer, 100))
        # An actual source revision can justify a new candidate; whether it
        # deserves interrupting the user still belongs to an unmodeled policy.
        changed = self.model.ingest("source", "new synthetic fact", "goal", "assist")
        self.model.claim(changed)
        self.model.publish(changed)
        self.assertIsNotNone(self.model.queue_notification(changed, 100))

    def test_forgotten_status_is_terminal_under_later_goal_and_purpose_changes(self):
        item = self.item()
        self.model.forget("source")
        self.model.set_goal("goal", "changed goal")
        self.model.set_purpose("assist", True)
        self.restart()
        self.assertEqual(self.model.describe(item)["validity"], "forgotten")
        self.assertFalse(self.model.describe(item)["result_available"])

    def test_prepared_approval_does_not_resume_after_restart(self):
        item = self.item()
        prep = self.model.prepare_draft(item, "https://synthetic.invalid")
        approval = self.model.approve(prep)
        self.restart()
        with self.assertRaises(Refused):
            self.model.dispatch_e1(prep, approval)
        self.assertEqual(self.model.describe(item)["executions"][0]["state"], "interrupted")

    def test_forget_during_external_work_rejects_late_result_and_old_pointer(self):
        item = self.item()
        notice = self.model.queue_notification(item, 0)
        payload = self.model.dispatch_notification(notice, 0)
        self.model.notification_result(notice, "submitted")
        prep = self.model.prepare_draft(item, "https://synthetic.invalid")
        approval = self.model.approve(prep)
        self.model.dispatch_e1(prep, approval)
        self.model.forget("source")
        self.assertFalse(self.model.e1_result(prep, "succeeded", "不可重新保存的旧正文"))
        self.restart()
        self.assertEqual(self.model.describe(item)["validity"], "forgotten")
        self.assertNotIn("下周", json.dumps(payload, ensure_ascii=False))
        with self.assertRaises(Refused):
            self.model.open_notification(notice)
        with self.assertRaises(Refused):
            self.model.recompute(self.model.describe(item)["case_id"])
        with self.assertRaises(Refused):
            self.model.ingest("source", "合成记录：下周讨论", "goal", "assist")
        self.assertIsNone(self.model.one("SELECT body FROM sources WHERE id='source'")[0])
        self.assertIsNone(self.model.one("SELECT body FROM executions WHERE id=?", (prep,))[0])
        self.assertIsNone(self.model.one("SELECT result FROM executions WHERE id=?", (prep,))[0])

    def test_source_change_during_work_prevents_publish(self):
        item = self.item(ready=False)
        self.model.claim(item)
        self.model.change_source("source", "new source")
        with self.assertRaises(Refused):
            self.model.publish(item)
        self.restart()
        self.assertEqual(self.model.describe(item)["work"], "invalidated")
        self.assertEqual(self.model.describe(item)["deliveries"], [])

    def test_handled_case_blocks_queued_delivery_and_inflight_result(self):
        item = self.item()
        notice = self.model.queue_notification(item, 0)
        prep = self.model.prepare_draft(item, "https://synthetic.invalid")
        approval = self.model.approve(prep)
        self.model.dispatch_e1(prep, approval)
        self.model.give_feedback(item, "done", uid())
        with self.assertRaises(Refused):
            self.model.dispatch_notification(notice, 0)
        self.assertFalse(self.model.e1_result(prep, "succeeded", "late"))

    def test_feedback_is_typed_scoped_and_idempotent(self):
        item = self.item()
        feedback = uid()
        self.model.give_feedback(item, "unhelpful", feedback)
        self.model.give_feedback(item, "unhelpful", feedback)
        state = self.model.describe(item)
        self.assertEqual(state["handling"], "open")
        self.assertEqual(state["validity"], "current")
        self.assertFalse(state["muted"])
        self.assertEqual(self.model.one("SELECT COUNT(*) FROM feedback")[0], 1)
        with self.assertRaises(Refused):
            self.model.give_feedback(item, "done", feedback)

    def test_later_cleanup_preserves_historical_result_acceptance(self):
        for change in ("done", "dismiss", "source_change", "forget"):
            with self.subTest(change=change):
                item = self.model.ingest(change, "synthetic fact", "goal", "assist")
                self.model.claim(item)
                self.model.publish(item)
                prep = self.model.prepare_draft(item, "https://synthetic.invalid")
                self.model.dispatch_e1(prep, self.model.approve(prep))
                self.assertTrue(self.model.e1_result(prep, "succeeded", "synthetic draft"))
                if change in {"done", "dismiss"}:
                    self.model.give_feedback(item, change, uid())
                elif change == "source_change":
                    self.model.change_source(change, "revised fact")
                else:
                    self.model.forget(change)
                self.restart()
                execution = self.model.one("SELECT * FROM executions WHERE id=?", (prep,))
                self.assertEqual(execution["state"], "succeeded")
                self.assertEqual(execution["accepted"], 1)
                self.assertIsNone(execution["result"])
                self.assertIsNone(execution["body"])
                self.assertFalse(self.model.describe(item)["result_available"])

    def test_live_owner_cannot_be_recovered_by_second_instance(self):
        item = self.item(ready=False)
        self.model.claim(item)
        with self.assertRaisesRegex(Refused, "live_owner"):
            Loop(self.path)
        self.assertEqual(self.model.describe(item)["work"], "running")

    def test_same_lease_cannot_repeat_recovery_and_reclaim_live_work(self):
        item = self.item(ready=False)
        self.model.claim(item)
        with self.assertRaisesRegex(Refused, "lease_already_recovered"):
            self.model.recover()
        with self.assertRaises(Refused):
            self.model.claim(item)
        self.assertEqual(self.model.describe(item)["work"], "running")
        self.restart()
        self.model.claim(item)
        self.assertEqual(self.model.describe(item)["work"], "running")

    def test_database_symlink_alias_cannot_bypass_live_owner_lease(self):
        item = self.item(ready=False)
        self.model.claim(item)
        alias = Path(self.tmp.name) / "alias.db"
        alias.symlink_to(self.path)
        with self.assertRaisesRegex(Refused, "live_owner"):
            Loop(alias)
        self.assertFalse(Path(str(alias) + ".lease").exists())
        self.assertEqual(self.model.describe(item)["work"], "running")

    def test_initialization_failure_releases_lease_and_database(self):
        item = self.item()
        self.model.close()
        with patch.object(Loop, "recover", side_effect=RuntimeError("synthetic initialization failure")):
            with self.assertRaisesRegex(RuntimeError, "synthetic initialization failure"):
                Loop(self.path)
        self.model = Loop(self.path)
        self.assertEqual(self.model.describe(item)["validity"], "current")

    def test_origin_and_approval_bound_to_one_preparation(self):
        item = self.item()
        a = self.model.prepare_draft(item, "https://one.invalid")
        b = self.model.prepare_draft(item, "https://two.invalid")
        approval = self.model.approve(a)
        with self.assertRaises(Refused):
            self.model.dispatch_e1(b, approval)
        self.assertEqual(self.model.dispatch_e1(a, approval)["origin"], "https://one.invalid")
        with self.assertRaises(Refused):
            self.model.dispatch_e1(a, approval)


def trace_examples():
    traces = []
    for scenario in ("三个输出协同", "用途撤回与旧批准", "稍后与通知未知结果", "遗忘与迟到草稿"):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "trace.db"
            model = Loop(path)
            try:
                model.set_goal("goal", "合成目标：沟通准备")
                model.set_purpose("assist", True)
                item = model.ingest("source", "合成记录，不是真实个人资料", "goal", "assist")
                steps = []

                def record(action, extra=None):
                    steps.append({"action": action, "state": model.describe(item), "detail": extra})

                record("事务提交输入和待处理工作")
                model.claim(item)
                model.publish(item)
                record("本地观察与收件箱同事务完成")
                case = model.describe(item)["case_id"]
                if scenario == "三个输出协同":
                    notice = model.queue_notification(item, 0)
                    payload = model.dispatch_notification(notice, 0)
                    model.notification_result(notice, "submitted")
                    record("系统收到通用提示，不视为用户读过", payload)
                    model.open_notification(notice)
                    prep = model.prepare_draft(item, "https://synthetic.invalid")
                    record("用户打开同一事项；本地准备待确认草稿")
                    approval = model.approve(prep)
                    model.dispatch_e1(prep, approval)
                    model.e1_result(prep, "succeeded", "合成草稿")
                    model.give_feedback(item, "copy", uid())
                    record("明确批准后模拟端点完成；复制仍不是已处理")
                    model.give_feedback(item, "done", uid())
                    record("用户明确标为已处理；事实有效性不变")
                elif scenario == "用途撤回与旧批准":
                    prep = model.prepare_draft(item, "https://synthetic.invalid")
                    approval = model.approve(prep)
                    record("批准绑定当前准备")
                    model.set_purpose("assist", False)
                    record("撤回用途使所有旧输出失效")
                    model.set_purpose("assist", True)
                    try:
                        model.dispatch_e1(prep, approval)
                    except Refused as error:
                        record("重新开启用途不恢复旧批准", str(error))
                elif scenario == "稍后与通知未知结果":
                    model.snooze(case, 1, 500)
                    item = model.recompute(case)
                    model.claim(item)
                    model.publish(item)
                    record("新版本保留同一事项的稍后时间")
                    record("稍后期间没有排入通知", model.queue_notification(item, 100))
                    notice = model.queue_notification(item, 500)
                    model.dispatch_notification(notice, 500)
                    model.close()
                    model = Loop(path)
                    record("在提交系统后中断并重开：结果未知")
                    try:
                        model.dispatch_notification(notice, 600)
                    except Refused as error:
                        record("未知结果不会自动重发", str(error))
                else:
                    notice = model.queue_notification(item, 0)
                    payload = model.dispatch_notification(notice, 0)
                    model.notification_result(notice, "submitted")
                    prep = model.prepare_draft(item, "https://synthetic.invalid")
                    approval = model.approve(prep)
                    model.dispatch_e1(prep, approval)
                    record("工作在途；通知只有通用文字和不透明指针", payload)
                    model.forget("source")
                    record("逻辑遗忘清除模型保存的来源与衍生正文")
                    accepted = model.e1_result(prep, "succeeded", "迟到的合成旧内容")
                    record("记录请求完成，但拒收迟到正文", {"accepted": accepted})
                    try:
                        model.open_notification(notice)
                    except Refused as error:
                        record("旧通知指针无法读取已遗忘内容", str(error))
                traces.append({"scenario": scenario, "steps": steps})
            finally:
                model.close()
    return {"scope": "synthetic_reference_only", "not_production": True, "scenarios": traces}


class RecordedResult(unittest.TextTestResult):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.records = []

    def addSuccess(self, test):
        super().addSuccess(test)
        self.records.append({"name": test.id(), "status": "passed"})

    def addFailure(self, test, err):
        super().addFailure(test, err)
        self.records.append({"name": test.id(), "status": "failed"})

    def addError(self, test, err):
        super().addError(test, err)
        self.records.append({"name": test.id(), "status": "error"})


if __name__ == "__main__":
    started = time.monotonic()
    suite = unittest.defaultTestLoader.loadTestsFromTestCase(LoopTests)
    result = unittest.TextTestRunner(verbosity=2, resultclass=RecordedResult).run(suite)
    report = {
        "scope": "synthetic_reference_only", "production_integration": False,
        "real_sqlite_transactions": True, "real_database_reopen": True,
        "physical_power_loss_tested": False, "network_calls": 0,
        "tests": result.testsRun, "failures": len(result.failures), "errors": len(result.errors),
        "passed": result.wasSuccessful(), "duration_seconds": round(time.monotonic()-started, 4),
        "checks": result.records,
    }
    here = Path(__file__).parent
    (here / "test_results.json").write_text(json.dumps(report, ensure_ascii=False, indent=2)+"\n", encoding="utf-8")
    if result.wasSuccessful():
        (here / "trace_examples.json").write_text(json.dumps(trace_examples(), ensure_ascii=False, indent=2)+"\n", encoding="utf-8")
    raise SystemExit(0 if result.wasSuccessful() else 1)
