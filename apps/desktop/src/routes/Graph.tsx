/**
 * 人脉图. Who Soul has seen you talk to, how often, and what says so.
 *
 * Two things this screen deliberately does not show. It shows no names: a
 * person's label is sealed text in the store and nothing on this path opens
 * the seal, so people are told apart by the leading characters of an
 * identifier digest. And it says nothing about what anybody is like — the
 * summary underneath is the core's sentences and the evidence rows behind
 * them, it carries 工作假设，非临床结论 from the core, and every sentence in
 * it was built on the other side of the IPC where `soul-policy`'s denylist
 * could see it. Where those sentences came from is on screen: counts, or a
 * rephrasing the user's own endpoint produced from those counts.
 */

import { useEffect, useState } from "react";

import {
  peopleGraph,
  personSummary,
  type PeopleGraph,
  type PersonNode,
  type PersonSummary,
  type Refusal,
  type TieEdge,
} from "../core";
import { asRefusal, Refused } from "../refusal";

/**
 * The band, in words. A band is how much was observed, not how much anything
 * is worth: the counts are on screen beside it so the reader can check.
 */
const BAND: Record<string, string> = {
  weak: "观察到的往来较少",
  moderate: "观察到的往来中等",
  strong: "观察到的往来较多",
};

/** The shapes `soul-graph` will admit to seeing. It names no relationships. */
const TIE_TYPE: Record<string, string> = {
  direct: "一对一说过话",
  group_only: "只在多人会话里出现",
  reciprocal: "两边都发过",
  one_sided: "只有一边发过",
};

/** Where the summary text came from. The codes are `PersonSummaryView.source`. */
const SUMMARY_SOURCE: Record<string, string> = {
  counts: "这一份是本机根据往来次数写的统计。",
  user_endpoint: "这一份是你自己的端点根据本机统计改写的。",
};

function words(source: Record<string, string>, key: string): string {
  return source[key] ?? key;
}

export function Graph(): React.JSX.Element {
  const [graph, setGraph] = useState<PeopleGraph | null>(null);
  const [refusal, setRefusal] = useState<Refusal | null>(null);
  const [summary, setSummary] = useState<PersonSummary | null>(null);
  const [summaryRefusal, setSummaryRefusal] = useState<Refusal | null>(null);

  useEffect(() => {
    let live = true;
    peopleGraph().then(
      (value) => {
        if (live) setGraph(value);
      },
      (error: unknown) => {
        if (live) setRefusal(asRefusal(error));
      },
    );
    return () => {
      live = false;
    };
  }, []);

  const summarize = (contactId: string): void => {
    setSummary(null);
    setSummaryRefusal(null);
    personSummary(contactId).then(
      (value) => setSummary(value),
      (error: unknown) => setSummaryRefusal(asRefusal(error)),
    );
  };

  if (refusal !== null) {
    return <Refused title="这一次没有读成" refusal={refusal} testId="graph-refusal-code" />;
  }

  if (graph === null) {
    return (
      <section className="panel" aria-busy="true">
        <p>正在读本机的人脉图…</p>
      </section>
    );
  }

  const others = graph.people.filter((person) => !person.is_you);

  return (
    <>
      <section className="panel" aria-labelledby="people-heading">
        <h2 id="people-heading">这台机器上看得见的人（{others.length}）</h2>
        <p className="muted">
          节点和边都是核心从你导入的往来记录里推出来的，不是谁填进去的。这一版不显示姓名：名字在库里是密封的，
          这个页面不去打开它。
        </p>
        {others.length === 0 ? (
          <p className="muted" data-testid="no-people">
            还没有可以显示的人。库里还没有往来记录可以推出关系：到「导入」页读一份你自己导出的聊天记录，
            人和关系会从那里面推出来。
          </p>
        ) : (
          <ul className="facts" data-testid="people-list">
            {others.map((person) => (
              <Person key={person.contact_id} person={person} onSummarize={summarize} />
            ))}
          </ul>
        )}
      </section>

      {graph.ties.length === 0 ? null : (
        <section className="panel" aria-labelledby="ties-heading">
          <h2 id="ties-heading">关系与证据（{graph.ties.length}）</h2>
          <ul className="facts" data-testid="ties-list">
            {graph.ties.map((tie) => (
              <Tie key={tie.relationship_id} tie={tie} />
            ))}
          </ul>
          {/*
            The outgoing half has to name what goes, not just that something
            does. `soul-draft`'s summary body is the whole set of statements
            the core derived from this edge — interaction count, active days,
            conversations, who sent how many, whether there was a one-to-one
            exchange, the date of the last one, and the band those counts put
            the tie in — under a fixed rewriting instruction. Saying only
            往来次数 understates that by five facts.
          */}
          <p className="muted" data-testid="local-only">
            {graph.third_party_data_is_local_only
              ? "别人的密封姓名和节点、边本身只留在本机，也不进研究预览。若你填了语言模型地址，按「看这个人的摘要」会把本机从这条边上算出来的那一整组统计发给那个地址：往来次数、有往来的天数与会话数、你和对方各发出多少条、有没有一对一说过话、最近一次往来的日期，还有本机按这些计数给出的关系档位，外加一句固定的改写要求——都是聚合出来的计数和结论，不带姓名，也不带任何人说过的原话，中间不会再问你一次。"
              : "有节点或边没有标成只留本机，请把这件事报告出来。"}
          </p>
        </section>
      )}

      {summaryRefusal === null ? null : (
        <Refused
          title="这个人的摘要没有出来"
          refusal={summaryRefusal}
          testId="summary-refusal-code"
        />
      )}

      {summary === null ? null : <Summary summary={summary} />}

      <p className="badge" data-testid="graph-notice">
        {graph.notice}
      </p>
    </>
  );
}

interface PersonProps {
  readonly person: PersonNode;
  readonly onSummarize: (contactId: string) => void;
}

function Person({ person, onSummarize }: PersonProps): React.JSX.Element {
  return (
    <li>
      <code>{person.identifier_hint}</code>
      <span className="muted">
        {" "}
        往来 {person.interaction_count} 次，{person.tie_count} 条关系
        {person.last_contact_utc === null ? "" : `，最近一次 ${person.last_contact_utc}`}
        {person.forgotten ? "（已被遗忘，只剩下墓碑）" : ""}
      </span>
      <button type="button" onClick={() => onSummarize(person.contact_id)}>
        看这个人的摘要
      </button>
    </li>
  );
}

interface TieProps {
  readonly tie: TieEdge;
}

function Tie({ tie }: TieProps): React.JSX.Element {
  return (
    <li>
      <span>
        {words(BAND, tie.band)}：往来 {tie.interaction_count} 次（发出 {tie.outgoing_count}，收到{" "}
        {tie.incoming_count}），{tie.conversation_count} 个会话，{tie.active_day_count} 天有往来，
        最近一次 {tie.last_contact_utc}。
      </span>
      <span className="muted">
        {" "}
        {tie.types.map((type) => words(TIE_TYPE, type)).join("、")}
      </span>
      <span className="muted" data-testid="tie-evidence">
        {" "}
        依据 {tie.evidence.length} 条证据（{tie.evidence.map((row) => row.kind).join("、")}）
      </span>
    </li>
  );
}

interface SummaryProps {
  readonly summary: PersonSummary;
}

/**
 * The summary the core wrote, rendered verbatim.
 *
 * The text is not assembled here on purpose: every line of it went through
 * `soul-policy`'s non-clinical check on the Rust side, and a sentence this
 * component built would not have.
 */
function Summary({ summary }: SummaryProps): React.JSX.Element {
  return (
    <section className="panel" aria-labelledby="summary-heading">
      <h2 id="summary-heading">这个人的摘要</h2>
      <p className="muted" data-testid="summary-source">
        {words(SUMMARY_SOURCE, summary.source)}
      </p>
      <p data-testid="summary-text">{summary.text}</p>
      <ul className="facts" data-testid="summary-points">
        {summary.points.map((point) => (
          <li key={point.statement}>
            {point.statement}
            <span className="muted">（依据 {point.evidence_ids.length} 条证据）</span>
          </li>
        ))}
      </ul>
      <p className="badge" data-testid="summary-notice">
        {summary.notice}
      </p>
    </section>
  );
}
