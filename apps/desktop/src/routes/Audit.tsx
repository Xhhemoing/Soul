/**
 * 审计. The chain, played back in order, with nothing in it but what happened.
 *
 * An entry has an action, a decision, a reason code, some ids and some counts.
 * It has no body, no prose and no name — `audit.schema.json` is
 * `additionalProperties: false` and declares none of those, so a stray one
 * fails validation on the Rust side long before it could reach this screen.
 * That is why the page can render every field it is handed: the shape is the
 * guarantee, not a filter written here.
 *
 * "Playback" means the chain is walked and checked, not merely listed. Each
 * entry carries the previous entry's hash, so the store's own verification
 * answers whether anything was edited or lifted out; `follows_previous` says
 * the same thing one line at a time, so a reader can see *where* a break is
 * rather than only that there is one.
 */

import { useEffect, useState } from "react";

import { auditChain, type AuditChain, type AuditEntry, type Refusal } from "../core";
import { asRefusal, Refused } from "../refusal";

/** `AuditAction`, in words. Every one of them is a thing, not a topic. */
const ACTION: Record<string, string> = {
  "consent.grant": "同意了一项能力",
  "collect.start": "开始采集",
  "collect.stop": "停止采集",
  "import.commit": "导入落库",
  "inference.write": "写入一条推断",
  "profile.correct": "纠正了档案",
  "memory.write": "写入一条记忆",
  "forget.execute": "执行遗忘",
  "draft.create": "生成草稿",
  "egress.request": "请求出网",
  "file.plan": "生成文件计划",
  "hitl.deny": "你当场拒绝了",
  "capability.reject": "能力被拒",
  "injection.blocked": "挡下了注入",
};

const DECISION: Record<string, string> = {
  allowed: "放行",
  denied: "拒绝",
  deferred: "等你决定",
  "n/a": "不涉及",
};

/** `EgressClass`. E0 and L have no code path in this build. */
const EGRESS: Record<string, string> = {
  none: "不出网",
  E0: "E0",
  E1: "E1",
  L: "L",
};

function words(source: Record<string, string>, key: string): string {
  return source[key] ?? key;
}

/** A hash, short enough to read and long enough to compare by eye. */
function shortHash(hash: string): string {
  return hash.slice(0, 12);
}

export function Audit(): React.JSX.Element {
  const [chain, setChain] = useState<AuditChain | null>(null);
  const [refusal, setRefusal] = useState<Refusal | null>(null);

  useEffect(() => {
    let live = true;
    auditChain().then(
      (value) => {
        if (live) setChain(value);
      },
      (error: unknown) => {
        if (live) setRefusal(asRefusal(error));
      },
    );
    return () => {
      live = false;
    };
  }, []);

  if (refusal !== null) {
    return <Refused title="审计链没有读出来" refusal={refusal} testId="audit-refusal-code" />;
  }

  if (chain === null) {
    return (
      <section className="panel" aria-busy="true">
        <p>正在回放审计链…</p>
      </section>
    );
  }

  return (
    <>
      <section className="panel" aria-labelledby="chain-heading">
        <h2 id="chain-heading">这条链现在是什么状态</h2>
        <p data-testid="audit-verified">
          {chain.verified
            ? `${chain.entries.length} 条记录，逐条对上了前一条的哈希。`
            : `链子对不上：${chain.verification_problem ?? "库没有说是哪里"}`}
        </p>
        <p className="muted" data-testid="audit-notice">
          {chain.notice}
        </p>
      </section>

      <section className="panel" aria-labelledby="entries-heading">
        <h2 id="entries-heading">回放（{chain.entries.length}）</h2>
        {chain.entries.length === 0 ? (
          <p className="muted" data-testid="no-audit-entries">
            链子上还没有记录。这台机器上还没有发生过需要记下来的事。
          </p>
        ) : (
          <ul className="facts" data-testid="audit-entries">
            {chain.entries.map((entry) => (
              <Entry key={entry.entry_id} entry={entry} />
            ))}
          </ul>
        )}
      </section>
    </>
  );
}

interface EntryProps {
  readonly entry: AuditEntry;
}

function Entry({ entry }: EntryProps): React.JSX.Element {
  return (
    <li data-testid={`audit-entry-${entry.seq}`}>
      <span>
        #{entry.seq} {entry.ts}｜{words(ACTION, entry.action)}｜{words(DECISION, entry.decision)}
      </span>
      {entry.reason_code === null ? null : (
        <span className="muted"> 理由码 {entry.reason_code}</span>
      )}
      {entry.egress_class === null ? null : (
        <span className="muted">｜{words(EGRESS, entry.egress_class)}</span>
      )}
      {entry.items === null && entry.bytes === null ? null : (
        <span className="muted">
          ｜{entry.items === null ? "" : `${entry.items} 项`}
          {entry.bytes === null ? "" : ` ${entry.bytes} 字节`}
        </span>
      )}
      {entry.subject_refs.length === 0 ? null : (
        <span className="muted">｜涉及 {entry.subject_refs.length} 个编号</span>
      )}
      {entry.plan_hash === null ? null : (
        <span className="muted">
          ｜计划 <code>{shortHash(entry.plan_hash)}</code>
        </span>
      )}
      <span className="muted" data-testid={`audit-link-${entry.seq}`}>
        ｜<code>{shortHash(entry.prev_hash)}</code> → <code>{shortHash(entry.entry_hash)}</code>
        {entry.follows_previous ? "" : "（这里接不上上一条）"}
      </span>
    </li>
  );
}
