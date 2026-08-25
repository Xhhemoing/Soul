/**
 * 研究预览. AC-20: what the research track would see, on screen and nowhere
 * else.
 *
 * There is no button on this page, and that is the whole design. v0.1 research
 * is preview-only, so the screen has nothing to save with: `core.ts` names
 * every command the shell has and none of them writes an export. The two
 * promises underneath are stronger than a missing button, though, and both are
 * checked on the Rust side rather than asserted here. `written_to_disk` is
 * typed as the literal `false` because `preview_manifest` is the only
 * constructor and it sets the field itself; `third_party_rows` is typed as the
 * literal `0` because `ZeroThirdPartyRows` refuses to deserialize anything
 * else, so a manifest carrying somebody else's row cannot be built at all.
 *
 * The rows themselves are counts and buckets. `export-manifest.schema.json`
 * has no field that could hold a body, a name or an identifier, which is why
 * this screen can render a row without deciding what is safe to show.
 */

import { useEffect, useState } from "react";

import { researchPreview, type Refusal, type ResearchPreview, type ResearchRow } from "../core";
import { asRefusal, Refused } from "../refusal";

/** The six columns `ExportField` allows, in words. */
const FIELD: Record<string, string> = {
  event_kind: "事件类型",
  time_bucket_utc: "时间桶（UTC）",
  duration_bucket: "时长桶",
  self_trait_axis: "你自己的特质轴",
  self_trait_band: "证据档位",
  aggregate_count: "聚合计数",
};

const BAND: Record<string, string> = {
  weak: "证据较少",
  moderate: "证据中等",
  strong: "证据较多",
};

function words(source: Record<string, string>, key: string): string {
  return source[key] ?? key;
}

/** An absent cell is a column this row's grouping did not produce. */
function cell(value: string | number | null): string {
  return value === null ? "—" : String(value);
}

export function Research(): React.JSX.Element {
  const [preview, setPreview] = useState<ResearchPreview | null>(null);
  const [refusal, setRefusal] = useState<Refusal | null>(null);

  useEffect(() => {
    let live = true;
    researchPreview().then(
      (value) => {
        if (live) setPreview(value);
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
    return <Refused title="预览没有出来" refusal={refusal} testId="research-refusal-code" />;
  }

  if (preview === null) {
    return (
      <section className="panel" aria-busy="true">
        <p>正在算这一份预览…</p>
      </section>
    );
  }

  return (
    <>
      <section className="panel" aria-labelledby="research-heading">
        <h2 id="research-heading">这一份预览是什么</h2>
        <ul className="facts">
          <li data-testid="research-on-screen-only">
            {preview.written_to_disk
              ? "这份清单声称自己已经落盘，请把这件事报告出来。"
              : "没有落盘：这份清单只在内存里算出来，关掉这一页它就没有了。"}
          </li>
          <li data-testid="research-third-party">
            别人的数据 {preview.third_party_rows} 行。查询里排除掉了{" "}
            {preview.third_party_rows_excluded} 行，别人的正文一律{preview.third_party_body}。
          </li>
          <li data-testid="research-rows">
            候选一共 {preview.candidate_rows_total} 行，这一页显示 {preview.rows.length} 行。
          </li>
          <li data-testid="research-own-withheld">
            你自己的数据里也排除掉了 {preview.deny_rows_excluded}{" "}
            行：它们存下来的时候标的研究口径不是「按小时计数」，这一页只显示按小时计数的那一种。
          </li>
          <li>
            清单编号：<code>{preview.manifest_id}</code>（{preview.export_kind}）
          </li>
        </ul>
        <p className="muted" data-testid="research-notice">
          {preview.notice}
        </p>
      </section>

      <section className="panel" aria-labelledby="research-fields-heading">
        <h2 id="research-fields-heading">能出现的列（{preview.fields.length}）</h2>
        <p className="muted" data-testid="research-fields">
          {preview.fields.map((field) => words(FIELD, field)).join("、")}
        </p>
        <p className="muted">
          就这几列。正文、姓名和任何指得到某个人的编号都不在里面，不是被过滤掉的，是清单里根本没有这样的位置。
        </p>
      </section>

      <section className="panel" aria-labelledby="research-table-heading">
        <h2 id="research-table-heading">行</h2>
        {/*
          An empty table says why the query came back empty and nothing more.
          This page only calls `research_preview`, so it cannot know whether
          collection is running or whether anything was ever imported;
          `candidate_rows_total` is the one number that separates "there was
          nothing to aggregate" from "everything found was excluded". Which
          exclusion did it is the next question, and there are two of them:
          saying 全部是别人的数据 when the owner's own imported rows are what
          got dropped would be the same kind of lie in the other direction.
        */}
        {preview.rows.length === 0 ? (
          <p className="muted" data-testid="no-research-rows">
            {preview.candidate_rows_total === 0
              ? "查询没有找到可以聚合的事件。"
              : `查询找到了 ${preview.candidate_rows_total} 行，${whyNothingShows(preview)}这一页因此没有可显示的行。`}
          </p>
        ) : (
          <table className="defaults" data-testid="research-table">
            <thead>
              <tr>
                <th scope="col">{words(FIELD, "event_kind")}</th>
                <th scope="col">{words(FIELD, "time_bucket_utc")}</th>
                <th scope="col">{words(FIELD, "duration_bucket")}</th>
                <th scope="col">{words(FIELD, "self_trait_axis")}</th>
                <th scope="col">{words(FIELD, "self_trait_band")}</th>
                <th scope="col">{words(FIELD, "aggregate_count")}</th>
              </tr>
            </thead>
            <tbody>
              {preview.rows.map((row, index) => (
                <Row key={rowKey(row, index)} row={row} />
              ))}
            </tbody>
          </table>
        )}
      </section>
    </>
  );
}

/**
 * Which exclusion emptied the table, named from the counts rather than
 * guessed. Both can be non-zero at once, and a count that is zero is left
 * unsaid rather than printed as a reason nothing was shown.
 */
function whyNothingShows(preview: ResearchPreview): string {
  const reasons: string[] = [];
  if (preview.third_party_rows_excluded > 0) {
    reasons.push(`别人的数据排除掉了 ${preview.third_party_rows_excluded} 行`);
  }
  if (preview.deny_rows_excluded > 0) {
    reasons.push(`你自己的数据里有 ${preview.deny_rows_excluded} 行不是按小时计数存下来的，也排除掉了`);
  }
  return reasons.length === 0 ? "" : `${reasons.join("，")}。`;
}

/**
 * A key for a row that has no identifier, because a row of research output is
 * not allowed to have one. Two rows with the same grouping would be the same
 * row, so the index is only ever a tiebreaker for an empty grouping.
 */
function rowKey(row: ResearchRow, index: number): string {
  return [row.event_kind, row.time_bucket_utc, row.duration_bucket, row.self_trait_axis, index]
    .map((part) => String(part))
    .join("|");
}

interface RowProps {
  readonly row: ResearchRow;
}

function Row({ row }: RowProps): React.JSX.Element {
  return (
    <tr>
      <td>{cell(row.event_kind)}</td>
      <td>{cell(row.time_bucket_utc)}</td>
      <td>{cell(row.duration_bucket)}</td>
      <td>{cell(row.self_trait_axis)}</td>
      <td>{row.self_trait_band === null ? "—" : words(BAND, row.self_trait_band)}</td>
      <td>{cell(row.aggregate_count)}</td>
    </tr>
  );
}
