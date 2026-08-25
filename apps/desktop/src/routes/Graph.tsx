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
 * could see it. Where those sentences came from is on screen, and the
 * distinction the wording has to keep is which of them this machine derived:
 * the points are counts with rows behind them, and the one line an endpoint
 * may have written is the endpoint's own sentence rather than a rewrite of
 * anything — the core cannot verify that it is one, so this screen does not
 * say it is.
 *
 * ## Correcting a tie
 *
 * The band under each tie is a working hypothesis, and constraint 10 rules out
 * one the user cannot overrule. So the three band words are buttons, the same
 * way the profile page's axis positions are, and the screen keeps showing what
 * the counts say after a correction: a lock the user can see the machine
 * disagreeing with is the only kind that is honest about what it did. What a
 * correction fixes is the one word; the counts beside it go on accumulating
 * and a later rebuild recomputes them without moving the band, which is why
 * there is a way back out to them as well.
 *
 * Agreeing with the band is a correction too. `soul_graph::correct_tie` locks
 * whatever band it is handed, including the one already in force, so pressing
 * the current word on an unlocked tie pins it: the counts go on accumulating
 * and the next rebuild leaves the word alone. Greying that button would have
 * left the only route to it 改成别的再改回来, which writes a `UserCorrection`
 * asserting a band the user never held. It is grey once the tie is locked,
 * because there the press really would restate a verdict already recorded.
 *
 * A correction is a write, so the whole graph comes back from the core rather
 * than being patched here — the band, the lock and the evidence rows behind
 * that edge all move, and a screen that updated one of them itself would be
 * guessing at the other two. Same shape as `Profile.tsx`, for the same reason.
 */

import { useEffect, useState } from "react";

import {
  correctTie,
  peopleGraph,
  personSummary,
  releaseTie,
  type PeopleGraph,
  type PersonNode,
  type PersonSummary,
  type Refusal,
  type TieEdge,
} from "../core";
import { asRefusal, Refused } from "../refusal";

/**
 * The band, in words. COPY_ZH allows only 弱 / 中等 / 强 for a档位; the
 * counts sit beside it so the reader can check the observation, not a
 * homemade scale (较少 / 较多).
 */
const BAND: Record<string, string> = {
  weak: "弱",
  moderate: "中等",
  strong: "强",
};

/**
 * The bands a correction may name, weakest first.
 *
 * Read off [`BAND`] rather than written out a second time: the three words are
 * COPY_ZH's whole vocabulary for a 档位, and a fourth key here would be a word
 * the core refuses anyway — `graph::band_named` is a closed set of the same
 * three.
 */
const BANDS: readonly string[] = Object.keys(BAND);

/** The shapes `soul-graph` will admit to seeing. It names no relationships. */
const TIE_TYPE: Record<string, string> = {
  direct: "一对一说过话",
  group_only: "只在多人会话里出现",
  reciprocal: "两边都发过",
  one_sided: "只有一边发过",
};

/**
 * Where the summary text came from. The codes are `PersonSummaryView.source`.
 *
 * `user_endpoint` used to read 这一份是你自己的端点根据本机统计改写的, which was
 * a claim nobody had checked: the core sends the counts and asks for a
 * rewrite, and an endpoint that answers with something else entirely was
 * getting its sentence displayed under this machine's provenance.
 * `soul-draft` now drops an answer that states a figure the counts do not or
 * that has nothing to do with them, and what survives that is still the
 * endpoint's own sentence — so this says whose sentence it is, and how little
 * was checked, rather than claiming it was derived here.
 */
const SUMMARY_SOURCE: Record<string, string> = {
  counts: "这一份是本机根据往来次数写的统计。",
  user_endpoint:
    "这一份里带证据的每一条仍然是本机根据往来次数算的；最后「整体来看」那一句是你自己的端点写的，本机只挡下了新出现的数字和跑题的回答，没有替你核对它说得对不对。",
};

function words(source: Record<string, string>, key: string): string {
  return source[key] ?? key;
}

export function Graph(): React.JSX.Element {
  const [graph, setGraph] = useState<PeopleGraph | null>(null);
  const [refusal, setRefusal] = useState<Refusal | null>(null);
  const [summary, setSummary] = useState<PersonSummary | null>(null);
  const [summaryRefusal, setSummaryRefusal] = useState<Refusal | null>(null);
  const [tieRefusal, setTieRefusal] = useState<Refusal | null>(null);
  const [busy, setBusy] = useState(false);

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

  /**
   * Both writes answer with the whole graph, so both are the same call.
   *
   * The person summary on screen goes with them. It was written from the
   * counts as they stood before this correction and it states the band among
   * them, so leaving it up would put a sentence from the machine's reading
   * underneath a lock that just overruled it — and the user has no way to
   * tell which of the two is current. It comes back by asking for it again.
   */
  const write = (change: Promise<PeopleGraph>): void => {
    setBusy(true);
    setTieRefusal(null);
    setSummary(null);
    setSummaryRefusal(null);
    change.then(
      (value) => {
        setGraph(value);
        setBusy(false);
      },
      (error: unknown) => {
        setTieRefusal(asRefusal(error));
        setBusy(false);
      },
    );
  };

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
          <p className="muted" data-testid="ties-explanation">
            档位是核心按下面那些计数算出来的，是工作假设，不是对谁的判断。你觉得哪一条不对，
            就按下面的 弱 / 中等 / 强 改；改过之后这一档就锁住了，以后再导入、再重算也不会覆盖你，
            计数照旧继续累加。觉得现在这一档就对、不想让以后重算动它，就按当前那一档把它锁住。
            想让计数重新说话，按「按计数重新算」。
          </p>
          {tieRefusal === null ? null : (
            <Refused title="这一档没有改成" refusal={tieRefusal} testId="tie-refusal-code" />
          )}
          <ul className="facts" data-testid="ties-list">
            {graph.ties.map((tie) => (
              <Tie
                key={tie.relationship_id}
                tie={tie}
                busy={busy}
                onCorrect={(band) => write(correctTie(tie.relationship_id, band))}
                onRelease={() => write(releaseTie(tie.relationship_id))}
              />
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

            The instruction is now the summary one rather than the drafting
            one (`soul_policy::e1::PERSON_SUMMARY_INSTRUCTION`), so naming what
            it asks for is naming something that is actually in the request
            body.
          */}
          <p className="muted" data-testid="local-only">
            {graph.third_party_data_is_local_only
              ? "别人的密封姓名和节点、边本身只留在本机，也不进研究预览。若你填了语言模型地址，按「看这个人的摘要」会把本机从这条边上算出来的那一整组统计发给那个地址：往来次数、有往来的天数与会话数、你和对方各发出多少条、有没有一对一说过话、最近一次往来的日期，还有本机按这些计数给出的关系档位，外加一句固定的系统指令（只许把这些计数改写成一段话，不许添新事实、不许改数字）——都是聚合出来的计数和结论，不带姓名，也不带任何人说过的原话，中间不会再问你一次。"
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
  readonly busy: boolean;
  readonly onCorrect: (band: string) => void;
  readonly onRelease: () => void;
}

/**
 * One tie: the band, the counts it was derived from, and the way to overrule
 * it.
 *
 * The machine's own reading stays on screen once the two disagree. The core
 * keeps `machine_band` beside the effective band exactly so this line can be
 * drawn without asking the scorer to run again, and a lock that hid what it
 * overruled would be the uncorrectable black box read backwards — the user
 * could no longer tell what the counts say about the edge they pinned.
 */
function Tie({ tie, busy, onCorrect, onRelease }: TieProps): React.JSX.Element {
  const disagrees =
    tie.locked_by_user && tie.machine_band !== null && tie.machine_band !== tie.band;

  return (
    <li data-testid={`tie-${tie.relationship_id}`}>
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
      {tie.locked_by_user ? (
        <span className="badge" data-testid={`tie-locked-${tie.relationship_id}`}>
          你改过这一档，重算不再动它
        </span>
      ) : null}
      {disagrees ? (
        <span className="muted" data-testid={`tie-machine-${tie.relationship_id}`}>
          {" "}
          机器按这些计数算的是「{words(BAND, tie.machine_band ?? "")}」，你改过之后它没有生效。
        </span>
      ) : null}
      <div className="switch-row">
        {BANDS.map((band) => (
          <button
            key={band}
            type="button"
            disabled={busy || (tie.locked_by_user && band === tie.band)}
            onClick={() => onCorrect(band)}
          >
            {words(BAND, band)}
          </button>
        ))}
        {tie.locked_by_user ? (
          <button type="button" disabled={busy} onClick={onRelease}>
            按计数重新算
          </button>
        ) : null}
      </div>
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
      {/*
        `reading`, because the core wrote this as lines and each line carries
        its own 依据 N 条记录 — or, for the one line an endpoint may have
        written, the sentence saying it has none. Collapsed into a paragraph,
        the claim and what is behind it stop being on the same line, which is
        the whole of AC-16's shape.
      */}
      <p className="reading" data-testid="summary-text">
        {summary.text}
      </p>
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
