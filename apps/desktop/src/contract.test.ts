/**
 * The rules the shell has to keep that no single component can be asked to
 * keep on its own.
 *
 * Three of them are structural — where IPC may be called from, what vocabulary
 * may appear on screen, and whether the TypeScript view of the core still
 * matches the Rust one. All three are the kind of thing that decays quietly,
 * so they are read off the files rather than agreed in review.
 */

import { readFileSync, readdirSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

import { COMMANDS } from "./core";
import {
  AUDIT_CHAIN_NOTICE,
  CLOUD_LABEL,
  COLLECT_DURATION_ONLY_NOTICE,
  COLLECT_NOT_OBSERVING_NOTICE,
  COLLECT_OFF_NOTICE,
  COLLECT_RUNNING_NOTICE,
  E1_PLAN_NOTICE,
  ENDPOINT_UNPARSABLE_NOTICE,
  FORGET_NOTICE,
  IMPORT_LOCAL_ONLY_NOTICE,
  LLM_ENDPOINT_SESSION_ONLY_NOTICE,
  NO_ANSWERS_NOTICE,
  NOT_SENT_NOTICE,
  QUESTIONS,
  READ_ONLY_NOTICE,
  RESEARCH_PREVIEW_NOTICE,
  TEMPLATE_NOTICE,
  WORKING_HYPOTHESIS_NOTICE,
} from "./test/fakeCore";

const SRC = dirname(fileURLToPath(import.meta.url));
const DESKTOP = join(SRC, "..");
const REPO = join(DESKTOP, "..", "..");
const SHELL_RS = join(REPO, "crates", "soulcore", "src", "commands", "shell.rs");
const DRAFT_RS = join(REPO, "crates", "soul-draft", "src", "draft.rs");
const CORE_DRAFT_RS = join(REPO, "crates", "soulcore", "src", "commands", "draft.rs");
const CORE_FILEPLAN_RS = join(REPO, "crates", "soulcore", "src", "commands", "fileplan.rs");
const CORE_IMPORT_RS = join(REPO, "crates", "soulcore", "src", "commands", "import.rs");
const CORE_MEMORY_RS = join(REPO, "crates", "soulcore", "src", "commands", "memory.rs");
const CORE_SESSION_RS = join(REPO, "crates", "soulcore", "src", "commands", "session.rs");
const CORE_STORE_RS = join(REPO, "crates", "soulcore", "src", "commands", "store.rs");
const CLINICAL_RS = join(REPO, "crates", "soul-policy", "src", "clinical.rs");
const QUESTIONNAIRE_RS = join(REPO, "crates", "soul-import", "src", "questionnaire.rs");
const TAURI_COMMANDS_RS = join(DESKTOP, "src-tauri", "src", "commands.rs");

/**
 * A `pub const NAME: &str = "…";`, including one broken across lines.
 *
 * A `\` at the end of a line inside a Rust string literal swallows the newline
 * *and* the indentation of the line after it, so the continuations are
 * left-trimmed here for the same reason: otherwise every wrapped constant in
 * the repository would look like a mismatch.
 */
function rustConstant(source: string, name: string): string {
  const pattern = new RegExp(`${name}: &str =\\s*"([\\s\\S]*?)";`);
  const match = pattern.exec(source);
  expect(match, `${name} is not declared the way this test reads constants`).not.toBeNull();
  return (match?.[1] ?? "").replace(/\\\r?\n\s*/g, "");
}

function sourceFiles(root: string, extensions: readonly string[]): string[] {
  const found: string[] = [];
  for (const entry of readdirSync(root, { withFileTypes: true })) {
    const path = join(root, entry.name);
    if (entry.isDirectory()) {
      found.push(...sourceFiles(path, extensions));
    } else if (extensions.some((extension) => entry.name.endsWith(extension))) {
      found.push(path);
    }
  }
  return found;
}

const isTestSupport = (path: string): boolean =>
  path.includes(`${join("src", "test")}`) || /\.test\.tsx?$/.test(path);

describe("壳与核心的边界", () => {
  /**
   * The shell holds no business logic, and the way that stays true is that
   * only one module can call the core at all. `eslint.config.js` says the same
   * thing; this says it again in a form that does not depend on anyone running
   * the linter.
   */
  it("只有 core.ts 直接引用 Tauri 的 API", () => {
    const offenders = sourceFiles(SRC, [".ts", ".tsx"])
      .filter((path) => path !== join(SRC, "core.ts") && !isTestSupport(path))
      .filter((path) => readFileSync(path, "utf8").includes("@tauri-apps/api"))
      .map((path) => relative(DESKTOP, path));

    expect(offenders).toEqual([]);
  });

  it("界面用的命令名和 src-tauri 注册的一模一样", () => {
    const registered = readFileSync(TAURI_COMMANDS_RS, "utf8");
    const declared = [...registered.matchAll(/#\[tauri::command\]\s*pub fn (\w+)\s*\(/g)].map(
      (match) => match[1],
    );
    expect(declared.sort()).toEqual([...Object.values(COMMANDS)].sort());
  });

  /**
   * 尚未启用 is a promise about the build, and it is asserted on both sides of
   * the IPC. If someone rewords one of them, this fails rather than letting
   * the shell and the core disagree about what the user was told.
   */
  it("云端开关的文案和 soulcore 里的常量是同一句话", () => {
    const rust = readFileSync(SHELL_RS, "utf8");
    const match = /CLOUD_NOT_YET_AVAILABLE_LABEL: &str = "([^"]+)"/.exec(rust);
    expect(match?.[1]).toBe(CLOUD_LABEL);
  });

  /**
   * 不会替你发送 is the promise WP10 exists to keep. The screen renders it out
   * of the value the core returns, so the only thing that could drift is the
   * double these tests run against — which would let the UI tests pass while
   * agreeing with nobody. Reading the constant off `soul-draft` closes that.
   */
  it("不发送与本机模板两句话和 soul-draft 里的常量一模一样", () => {
    const rust = readFileSync(DRAFT_RS, "utf8");
    expect(rustConstant(rust, "NOT_SENT_NOTICE")).toBe(NOT_SENT_NOTICE);
    expect(rustConstant(rust, "TEMPLATE_NOTICE")).toBe(TEMPLATE_NOTICE);
  });

  /**
   * The three sentences WP13's second slice put on screen. Each one is a
   * promise the Rust tests hold — v0.1 writes no files, a generation request
   * is described before it runs, and nothing here is a clinical conclusion —
   * and each reaches the user through the double these tests run against. A
   * double that softened one of them would let every UI test pass while the
   * screen said something the core never agreed to.
   */
  it("只读、确认与非临床三句话都和核心里的常量一模一样", () => {
    expect(rustConstant(readFileSync(CORE_FILEPLAN_RS, "utf8"), "READ_ONLY_NOTICE")).toBe(
      READ_ONLY_NOTICE,
    );
    expect(rustConstant(readFileSync(CORE_DRAFT_RS, "utf8"), "E1_PLAN_NOTICE")).toBe(
      E1_PLAN_NOTICE,
    );
    expect(
      rustConstant(readFileSync(CLINICAL_RS, "utf8"), "WORKING_HYPOTHESIS_NOTICE"),
    ).toBe(WORKING_HYPOTHESIS_NOTICE);
  });

  /**
   * The four sentences the last four routes put on screen. Each one is a
   * promise the Rust tests hold — what a forget destroys, that research never
   * lands on disk, that the chain holds no content, and that a blank
   * questionnaire is refused rather than reported as an intake — and each
   * reaches the user through the double these tests run against.
   */
  it("遗忘、研究、审计与空问卷四句话都和核心里的常量一模一样", () => {
    expect(rustConstant(readFileSync(CORE_MEMORY_RS, "utf8"), "FORGET_NOTICE")).toBe(
      FORGET_NOTICE,
    );
    const store = readFileSync(CORE_STORE_RS, "utf8");
    expect(rustConstant(store, "RESEARCH_PREVIEW_NOTICE")).toBe(RESEARCH_PREVIEW_NOTICE);
    expect(rustConstant(store, "AUDIT_CHAIN_NOTICE")).toBe(AUDIT_CHAIN_NOTICE);
    expect(rustConstant(readFileSync(CORE_SESSION_RS, "utf8"), "NO_ANSWERS_NOTICE")).toBe(
      NO_ANSWERS_NOTICE,
    );
  });

  /**
   * The sentence the import screen shows about where a picked file goes. It is
   * a promise about this build — read locally, sealed locally, never obeyed —
   * so it is the core's own words rather than something the page composed, and
   * the double these tests run against has to say the same thing.
   */
  /**
   * The four sentences the collection page renders. The first is
   * PRODUCT_LOCK's promise about this slice — duration only, no window titles
   * — and the other three are the only three states collection has. All four
   * are the core's own words, because "什么都没有在采" is a claim about a
   * consent ledger and a thread, and only the core can see either.
   */
  it("采集那四句话都和核心里的常量一模一样", () => {
    const session = readFileSync(CORE_SESSION_RS, "utf8");
    expect(rustConstant(session, "COLLECT_DURATION_ONLY_NOTICE")).toBe(
      COLLECT_DURATION_ONLY_NOTICE,
    );
    expect(rustConstant(session, "COLLECT_OFF_NOTICE")).toBe(COLLECT_OFF_NOTICE);
    expect(rustConstant(session, "COLLECT_RUNNING_NOTICE")).toBe(COLLECT_RUNNING_NOTICE);
    expect(rustConstant(session, "COLLECT_NOT_OBSERVING_NOTICE")).toBe(
      COLLECT_NOT_OBSERVING_NOTICE,
    );
  });

  /**
   * The two sentences the endpoint form renders. One is what the address is —
   * this run only, contacted by nothing until a generation is approved — and
   * the other is what the core says when what it was handed is not an address.
   * Both are claims about `Session::set_user_endpoint` rather than about this
   * page, so both are read off the Rust side.
   */
  it("端点那两句话都和核心里的常量一模一样", () => {
    expect(
      rustConstant(readFileSync(SHELL_RS, "utf8"), "LLM_ENDPOINT_SESSION_ONLY_NOTICE"),
    ).toBe(LLM_ENDPOINT_SESSION_ONLY_NOTICE);
    expect(
      rustConstant(readFileSync(CORE_SESSION_RS, "utf8"), "ENDPOINT_UNPARSABLE_NOTICE"),
    ).toBe(ENDPOINT_UNPARSABLE_NOTICE);
  });

  it("导入那句话和核心里的常量一模一样", () => {
    expect(
      rustConstant(readFileSync(CORE_IMPORT_RS, "utf8"), "IMPORT_LOCAL_ONLY_NOTICE"),
    ).toBe(IMPORT_LOCAL_ONLY_NOTICE);
  });

  /**
   * `soul_import::questionnaire::QUESTIONS` is the canonical list — the
   * recorder validates against it and `soul-profile` builds its own
   * questionnaire from it rather than keeping a second one. The wizard draws
   * whatever the core sends, so what this pins is the double: a twelfth
   * question, or an id changed on the Rust side, has to fail here rather than
   * leave the wizard's tests passing against a list nobody asks.
   */
  it("向导那份问卷和 soul-import 的正典清单是同一份", () => {
    const rust = readFileSync(QUESTIONNAIRE_RS, "utf8");
    const declared = [...rust.matchAll(/key: "(q\.[a-z_.]+)"/g)].map((match) => match[1]);

    expect(declared).toHaveLength(11);
    expect(QUESTIONS.map((question) => question.question_id)).toEqual(declared);
    // Three of them are text boxes, and a text box's answer is sealed.
    expect(QUESTIONS.filter((question) => question.prose)).toHaveLength(3);
    for (const question of QUESTIONS) {
      expect(question.options).toHaveLength(question.prose ? 0 : 3);
    }
  });
});

describe("起草只写不发", () => {
  /**
   * The shell can only do what it has a command for, and `core.ts` is the one
   * place a command is named. So the check for "there is no send button" is a
   * check on that list: nothing on it is a verb that delivers anything, and
   * `src-tauri/tests/command_surface.rs` holds the Rust side to the same list.
   */
  it("界面能调用的命令里没有一个是发送", () => {
    for (const command of Object.values(COMMANDS)) {
      expect(command).not.toMatch(/send|deliver|dispatch|post_|reply_to|message_/);
    }
  });

  /**
   * The words themselves, across the tree the user actually sees. A button
   * that offered to send would have to say so somewhere, and this is where.
   */
  it("界面上不出现替用户发送的说法", () => {
    const hits: string[] = [];
    for (const path of sourceFiles(SRC, [".ts", ".tsx"])) {
      if (isTestSupport(path)) continue;
      for (const [index, line] of readFileSync(path, "utf8").split("\n").entries()) {
        // The core's own notice says 不会替你发送, and quoting it is the point.
        if (/替你发送|替我发送|直接发送|发送给|发出去给/.test(line) && !line.includes("不会")) {
          hits.push(`${relative(DESKTOP, path)}:${index + 1}`);
        }
      }
    }
    expect(hits).toEqual([]);
  });
});

describe("界面用词", () => {
  /**
   * `xtask denylist-audit` reads the same file but only scans `crates/`. The
   * words it bans are about what the product says to the user, and the place
   * the product says things to the user is here, so the UI tree checks itself
   * against the same list.
   */
  it("不出现诊断词与量表词", () => {
    const terms = readFileSync(join(REPO, "fixtures", "denylist", "diagnostic_terms.txt"), "utf8")
      .split("\n")
      .map((line) => line.trim())
      .filter((line) => line !== "" && !line.startsWith("#"))
      .map((line) => line.toLowerCase());
    expect(terms.length).toBeGreaterThan(50);

    const hits: string[] = [];
    for (const path of sourceFiles(SRC, [".ts", ".tsx", ".css"])) {
      if (isTestSupport(path)) continue;
      const text = readFileSync(path, "utf8").toLowerCase();
      for (const term of terms) {
        const isAscii = /^[\x20-\x7e]+$/.test(term);
        const pattern = isAscii
          ? new RegExp(`(^|[^a-z0-9_])${term.replace(/[.*+?^${}()|[\]\\-]/g, "\\$&")}($|[^a-z0-9_])`)
          : null;
        const found = pattern === null ? text.includes(term) : pattern.test(text);
        if (found) hits.push(`${relative(DESKTOP, path)}: ${term}`);
      }
    }

    expect(hits).toEqual([]);
  });
});
