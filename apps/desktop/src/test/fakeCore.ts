/**
 * A stand-in for soulcore, wired into the real `@tauri-apps/api` transport.
 *
 * The shell's tests run the actual `core.ts` — argument marshalling included —
 * and only the far side of the IPC is faked. The values below mirror
 * `crates/soulcore/src/commands/shell.rs`; `contract.test.ts` reads that file
 * and fails if the two drift apart, which is what keeps this double from
 * quietly becoming a nicer core than the real one. Draft and file-plan
 * notices are pinned the same way against `draft.rs` and `fileplan.rs`.
 */

import { mockIPC } from "@tauri-apps/api/mocks";
import { vi } from "vitest";

import type { CloudNotice, ConfigSnapshot, DraftView, FilePlanView } from "../core";

export const CLOUD_LABEL = "尚未启用";

/**
 * The sentence `KEY_FILE_NOT_PROTECTED_EXPLANATION` holds in
 * `crates/soulcore/src/commands/shell.rs`, copied here so the shell's tests can
 * assert the screen shows it word for word. `contract.test.ts` reads the Rust
 * constant and fails if this copy drifts, which is what stops the double from
 * quietly telling the user a kinder story than the build can back up.
 */
export const KEY_PROTECTION =
  "数据库的密钥现在放在数据目录里的一个明文文件 soul-test-keys.bin，没有交给 Windows 的 DPAPI —— 那一段还没有实现。所以能读到这个目录的人，就能打开你的库：在 DPAPI 补上之前，这台电脑的登录口令是唯一的一道门。";

/**
 * A path the double refuses, and the words it refuses with.
 *
 * The real core refuses by looking at the disk. jsdom has no disk, so what is
 * faked here is the *shape* of a refusal — a `reason` and a sentence the core
 * wrote. Which paths are refusable is asserted in `crates/soulcore` and over
 * the real IPC in `apps/desktop/src-tauri/tests/ipc_roundtrip.rs`.
 */
export const REFUSED_ROOT = "D:\\没有这个目录";
export const REFUSED_ROOT_MESSAGE = `这个路径不存在：${REFUSED_ROOT}`;

/**
 * A directory the double refuses to scan, and the words it refuses with.
 *
 * Copied from `FilePlanError::PathNotAuthorized` with an empty root list: the
 * real core never echoes the path it refused, so neither does this. The
 * constant is a path the UI tests can type; it must not appear in the
 * message, or a page that rendered the refusal would be confirming the path.
 */
export const REFUSED_TARGET = "D:\\没有授权这个目录";
export const REFUSED_TARGET_MESSAGE =
  "that path is not inside any authorized root (no directory has been authorized)";

/**
 * The sentence `DRAFT_NEVER_SENT_EXPLANATION` holds in
 * `crates/soulcore/src/commands/draft.rs`. `contract.test.ts` reads the Rust
 * constant and fails if this copy drifts.
 */
export const DRAFT_NEVER_SENT_EXPLANATION =
  "这是一份草稿。本产品没有把它发出去的代码路径：要不要用、发给谁、什么时候发，都由你自己决定。";

export const TEMPLATE_ROUTE_LABEL = "本机模板写成，没有任何字节出网。";

export const EMPTY_PASTE_EXPLANATION =
  "粘贴框是空的。先把要回复的那段话贴进来，再让它起草。";

/**
 * The sentence `PLAN_PREVIEW_ONLY_EXPLANATION` holds in
 * `crates/soulcore/src/commands/fileplan.rs`, with the line-wrap undone the
 * way `rustStringConstant` undoes it.
 */
export const PLAN_PREVIEW_ONLY_EXPLANATION =
  "以下只是建议。本版本没有执行它的代码路径：一个文件都不会被移动、改名或删除，磁盘上的东西保持原样。真正动手整理是 v0.1.1 的事。";

export const CLOSED_CLOUD: CloudNotice = {
  state: "not_yet_available",
  label: CLOUD_LABEL,
  enabled: false,
  performs_network_request: false,
  explanation:
    "v0.1 没有云端出网的代码路径。开关留在这里是为了让你看见它默认是关的，点它不会发出任何请求，也不会把任何内容送出本机。",
};

export const CLOSED_SNAPSHOT: ConfigSnapshot = {
  collect_enabled: false,
  cloud: CLOSED_CLOUD,
  llm_endpoint_configured: false,
  authorized_root_count: 0,
  fully_closed: true,
  open_capabilities: [],
  kek_protected: false,
  key_protection: KEY_PROTECTION,
};

export const CLOSED_DRAFT: DraftView = {
  text: "[第三人正文已占位]",
  route: "template",
  route_label: TEMPLATE_ROUTE_LABEL,
  turns: 1,
  third_party_turns: 1,
  placeheld_turns: 1,
  carries_exempted_original: false,
  never_sent: true,
  notice: DRAFT_NEVER_SENT_EXPLANATION,
};

export const CLOSED_FILEPLAN: FilePlanView = {
  scan_id: "00000000-0000-7000-8000-000000000001",
  file_count: 2,
  dir_count: 1,
  skipped_escaping_links: 0,
  entry_count: 1,
  group_count: 1,
  move_count: 0,
  rename_count: 0,
  entries: [
    {
      source_rel: "notes.xlsx",
      action: "group",
      action_label: "归类",
      target_rel: "spreadsheets/notes.xlsx",
    },
  ],
  written_to_disk: false,
  notice: PLAN_PREVIEW_ONLY_EXPLANATION,
};

export interface RecordedCall {
  readonly cmd: string;
  readonly payload: unknown;
}

export interface FakeCore {
  /** Every command the shell sent, in order. */
  readonly calls: RecordedCall[];
  callsTo(cmd: string): RecordedCall[];
}

/**
 * Install the double.
 *
 * `complete_wizard` refuses an unacknowledged wizard the way the core does,
 * because the UI has to handle the refusal; everything else is a constant,
 * because everything else in this work package is.
 */
export function installFakeCore(snapshot: ConfigSnapshot = CLOSED_SNAPSHOT): FakeCore {
  const calls: RecordedCall[] = [];
  const roots: string[] = [];

  mockIPC((cmd, payload) => {
    calls.push({ cmd, payload });
    switch (cmd) {
      case "config_snapshot":
        return { ...snapshot, authorized_root_count: roots.length };
      case "authorized_roots":
        return [...roots];
      case "authorize_root": {
        // The core resolves the path and answers with a whole snapshot. The
        // double keeps both halves: a UI that ignored the answer and counted
        // its own inputs would still pass if this returned nothing.
        const path = String((payload as { path?: unknown }).path ?? "").trim();
        if (path === "" || path === REFUSED_ROOT) {
          throw {
            reason: path === "" ? "empty" : "not_found",
            message: path === "" ? "还没有填目录。" : REFUSED_ROOT_MESSAGE,
          };
        }
        roots.push(path);
        return {
          ...snapshot,
          authorized_root_count: roots.length,
          fully_closed: false,
          open_capabilities: ["authorized_roots"],
        };
      }
      case "complete_wizard": {
        const answers = (payload as { answers?: { acknowledged_defaults_are_off?: boolean } })
          .answers;
        if (answers?.acknowledged_defaults_are_off !== true) {
          throw "the wizard was not acknowledged, so there is nothing to finish";
        }
        return snapshot;
      }
      case "cloud_toggle":
        // Whatever was requested, the answer is the notice. This is the whole
        // of AC-22 on the core side, and the shell must not improve on it.
        return snapshot.cloud;
      case "draft_view": {
        const pasted = (payload as { pasted?: unknown }).pasted;
        const items = Array.isArray(pasted) ? pasted.map((item) => String(item)) : [];
        if (items.length === 0 || items.every((item) => item.trim() === "")) {
          throw {
            reason: "empty_paste",
            code: null,
            message: EMPTY_PASTE_EXPLANATION,
          };
        }
        return CLOSED_DRAFT;
      }
      case "fileplan_view": {
        const target = String((payload as { target?: unknown }).target ?? "");
        if (target === "" || target === REFUSED_TARGET) {
          throw {
            reason: "refused",
            code: "PATH_NOT_AUTHORIZED",
            message: REFUSED_TARGET_MESSAGE,
          };
        }
        return CLOSED_FILEPLAN;
      }
      default:
        throw `the shell called a command the core does not have: ${cmd}`;
    }
  });

  return {
    calls,
    callsTo: (cmd: string) => calls.filter((call) => call.cmd === cmd),
  };
}

/**
 * Fail the test if anything in the shell tries to reach the network.
 *
 * jsdom has no real sockets, so this is about intent rather than packets: it
 * catches a component that grew a `fetch` on the way to satisfying a design
 * review. Every stub is a `vi.stubGlobal`, so `unstubGlobals` puts the
 * originals back between tests. Returns the list of attempts, which stays
 * empty in a passing run.
 */
export function forbidNetwork(): string[] {
  const attempts: string[] = [];
  const record =
    (what: string) =>
    (...args: unknown[]) => {
      attempts.push(`${what}(${args.map((argument) => String(argument)).join(", ")})`);
      throw new Error(`the shell must not reach the network, but it called ${what}`);
    };

  vi.stubGlobal("fetch", record("fetch"));
  vi.stubGlobal(
    "XMLHttpRequest",
    class {
      open = record("XMLHttpRequest.open");
      send = record("XMLHttpRequest.send");
    },
  );
  vi.stubGlobal(
    "WebSocket",
    class {
      constructor(...args: unknown[]) {
        record("WebSocket")(...args);
      }
    },
  );
  vi.stubGlobal(
    "EventSource",
    class {
      constructor(...args: unknown[]) {
        record("EventSource")(...args);
      }
    },
  );
  vi.stubGlobal("navigator", { ...navigator, sendBeacon: record("navigator.sendBeacon") });

  return attempts;
}
