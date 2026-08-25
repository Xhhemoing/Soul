/**
 * 研究预览: AC-20 on the screen a person reads it on.
 *
 * Three claims, and none of them is "the button is hidden". Nothing was
 * written to disk, no row is about anybody else, and there is no command on
 * this page that could produce a file — the last one is checked against
 * `core.ts`'s own list, which is the only place the shell names a command.
 */

import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { Research } from "./Research";
import { COMMANDS } from "../core";
import { denylistHits, diagnosticTerms, renderedText } from "../test/denylist";
import {
  aResearchPreview,
  forbidNetwork,
  installFakeCore,
  RESEARCH_PREVIEW_NOTICE,
  type FakeCoreOptions,
} from "../test/fakeCore";

async function open(options: FakeCoreOptions = {}) {
  const core = installFakeCore(options);
  render(<Research />);
  await screen.findByRole("heading", { name: "这一份预览是什么" });
  return core;
}

describe("研究预览页", () => {
  it("屏幕上写着没有落盘，并且说清楚关掉就没了", async () => {
    await open();

    expect(screen.getByTestId("research-on-screen-only")).toHaveTextContent("没有落盘");
    expect(screen.getByTestId("research-notice")).toHaveTextContent(RESEARCH_PREVIEW_NOTICE);
  });

  /**
   * 0 is not a number this screen writes: the manifest carries it, and
   * `ZeroThirdPartyRows` refuses to deserialize anything else. The excluded
   * count beside it is what shows the exclusion actually ran.
   */
  it("别人的数据是 0 行，而且看得见排除掉了几行", async () => {
    await open();

    expect(screen.getByTestId("research-third-party")).toHaveTextContent("别人的数据 0 行");
    expect(screen.getByTestId("research-third-party")).toHaveTextContent("排除掉了 2 行");
    expect(screen.getByTestId("research-third-party")).toHaveTextContent("excluded");
  });

  it("行是计数和桶，没有一列能放正文或姓名", async () => {
    await open();

    expect(screen.getByTestId("research-fields")).toHaveTextContent("事件类型");
    expect(screen.getByTestId("research-fields")).toHaveTextContent("聚合计数");
    const table = screen.getByTestId("research-table");
    expect(table).toHaveTextContent("app_usage");
    expect(table).toHaveTextContent("2026-08-20T09:00:00Z");
  });

  it("没有可聚合的事件时说清楚为什么是空的", async () => {
    await open({ research: () => aResearchPreview({ rows: [], candidate_rows_total: 0 }) });

    expect(screen.getByTestId("no-research-rows")).toHaveTextContent("采集没有打开");
    expect(screen.queryByTestId("research-table")).toBeNull();
  });

  /**
   * The absence of an export is a fact about the command list rather than
   * about this component's layout: `core.ts` is the only place the shell names
   * a command, and none of them writes anything.
   */
  it("界面上没有导出按钮，能调用的命令里也没有一个会写文件", async () => {
    const attempts = forbidNetwork();
    await open();

    expect(screen.queryAllByRole("button")).toEqual([]);
    for (const command of Object.values(COMMANDS)) {
      expect(command).not.toMatch(/export|write_|save|download|upload/);
    }
    expect(attempts).toEqual([]);
  });

  it("渲染出来的预览里没有一个诊断词或量表词", async () => {
    const terms = diagnosticTerms();
    expect(terms.length).toBeGreaterThan(50);

    await open({
      research: () =>
        aResearchPreview({
          fields: ["self_trait_axis", "self_trait_band", "aggregate_count"],
          rows: [
            {
              event_kind: null,
              time_bucket_utc: null,
              duration_bucket: null,
              self_trait_axis: "curiosity",
              self_trait_band: "moderate",
              aggregate_count: 3,
            },
          ],
        }),
    });

    expect(denylistHits(renderedText(), terms)).toEqual([]);
  });

  it("读不到库的时候给出理由码，而不是一张空表", async () => {
    installFakeCore({
      research: () => {
        throw {
          reason_code: "STORE_UNAVAILABLE",
          explanation: "数据库这次没有打开，所以算不出预览。",
        };
      },
    });
    render(<Research />);

    expect(await screen.findByTestId("research-refusal-code")).toHaveTextContent(
      "STORE_UNAVAILABLE",
    );
    expect(screen.queryByTestId("research-table")).toBeNull();
  });
});
