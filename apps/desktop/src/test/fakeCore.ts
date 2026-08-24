/**
 * A stand-in for soulcore, wired into the real `@tauri-apps/api` transport.
 *
 * The shell's tests run the actual `core.ts` — argument marshalling included —
 * and only the far side of the IPC is faked. The values below mirror
 * `crates/soulcore/src/commands/shell.rs`; `contract.test.ts` reads that file
 * and fails if the two drift apart, which is what keeps this double from
 * quietly becoming a nicer core than the real one.
 */

import { mockIPC } from "@tauri-apps/api/mocks";
import { vi } from "vitest";

import type {
  CloudNotice,
  ConfigSnapshot,
  Draft,
  DraftNotices,
  E1DraftPlan,
  FilesView,
  PeopleGraph,
  PersonSummary,
  PlanPreview,
  SessionStatus,
} from "../core";

export const CLOUD_LABEL = "尚未启用";

/** `soul_draft::draft::NOT_SENT_NOTICE`, checked by `contract.test.ts`. */
export const NOT_SENT_NOTICE = "这是草稿。Soul 不会替你发送，检查和修改之后由你自己发出去。";

/** `soul_draft::draft::TEMPLATE_NOTICE`. */
export const TEMPLATE_NOTICE = "本次没有用模型：草稿由本机确定性语气模板生成，没有任何内容离开本机。";

/** `soulcore::commands::draft::E1_PLAN_NOTICE`, checked by `contract.test.ts`. */
export const E1_PLAN_NOTICE =
  "确认之后，只有下面这些内容会发到你自己配置的模型端点，用来生成草稿。第三人正文默认已占位。草稿生成之后仍然由你自己决定要不要发出去，Soul 不会替你发送。";

/** `soulcore::commands::fileplan::READ_ONLY_NOTICE`, checked the same way. */
export const READ_ONLY_NOTICE =
  "v0.1 只做只读扫描与计划预览，不会移动、重命名或删除任何文件。受控目录整理的执行与撤销是 v0.1.1 的事。";

/** `soul_policy::clinical::WORKING_HYPOTHESIS_NOTICE`. */
export const WORKING_HYPOTHESIS_NOTICE = "工作假设，非临床结论";

export const DRAFT_NOTICES: DraftNotices = {
  not_sent: NOT_SENT_NOTICE,
  e1_plan: E1_PLAN_NOTICE,
  can_send: false,
};

/** A session that has been through the wizard and has its store open. */
export const OPEN_SESSION: SessionStatus = {
  wizard_completed: true,
  store_opened: true,
  key_protection: "developer_key_file",
  store_notice: "这是开发构建：数据库密钥放在库旁边的种子文件里，没有平台密钥保护。",
  config_problem: null,
};

export const NO_ROOTS: FilesView = {
  read_only_notice: READ_ONLY_NOTICE,
  executable_in_this_version: false,
  roots: [],
  unavailable_roots: [],
};

/** One directory, and the plan a scan of it produced. */
export function aFilesView(overrides: Partial<FilesView> = {}): FilesView {
  return { ...NO_ROOTS, roots: [{ path: "/home/li/下载" }], ...overrides };
}

export function aPlanPreview(overrides: Partial<PlanPreview> = {}): PlanPreview {
  return {
    root: "/home/li/下载",
    plan_hash: "b3".repeat(32),
    directory_snapshot: "c4".repeat(32),
    disk_unchanged: true,
    executable_in_this_version: false,
    read_only_notice: READ_ONLY_NOTICE,
    scanned_entries: 3,
    skipped_entries: 1,
    truncated: false,
    moves: [
      { from: "预算.csv", to: "表格/预算.csv", kind: "spreadsheet", kind_label: "表格", size_bytes: 12 },
      { from: "photo.jpg", to: "图片/photo.jpg", kind: "image", kind_label: "图片", size_bytes: 8 },
    ],
    left_alone: [
      { path: "mystery.qqq", reason: "unknown_kind", explanation: "认不出这是什么类型的文件" },
    ],
    ...overrides,
  };
}

export const EMPTY_GRAPH: PeopleGraph = {
  self_contact_id: null,
  people: [],
  ties: [],
  notice: WORKING_HYPOTHESIS_NOTICE,
  third_party_data_is_local_only: true,
};

/**
 * A graph with one other person in it.
 *
 * Shaped the way `soulcore::commands::graph::PeopleGraphView` shapes one:
 * counts, an identifier digest, and no name anywhere.
 */
export function aPeopleGraph(overrides: Partial<PeopleGraph> = {}): PeopleGraph {
  return {
    self_contact_id: "0192f000-0000-7000-8000-000000000001",
    people: [
      {
        contact_id: "0192f000-0000-7000-8000-000000000001",
        is_you: true,
        identifier_hint: "aaaaaaaa",
        interaction_count: 6,
        last_contact_utc: "2026-08-20T09:00:00Z",
        tie_count: 1,
        forgotten: false,
      },
      {
        contact_id: "0192f000-0000-7000-8000-000000000002",
        is_you: false,
        identifier_hint: "bbbbbbbb",
        interaction_count: 6,
        last_contact_utc: "2026-08-20T09:00:00Z",
        tie_count: 1,
        forgotten: false,
      },
    ],
    ties: [
      {
        relationship_id: "0192f000-0000-7000-8000-00000000000a",
        from_contact_id: "0192f000-0000-7000-8000-000000000001",
        to_contact_id: "0192f000-0000-7000-8000-000000000002",
        types: ["direct", "reciprocal"],
        band: "moderate",
        interaction_count: 6,
        outgoing_count: 3,
        incoming_count: 3,
        conversation_count: 2,
        active_day_count: 4,
        first_contact_utc: "2026-07-01T08:00:00Z",
        last_contact_utc: "2026-08-20T09:00:00Z",
        local_only: true,
        evidence: [
          { evidence_id: "0192f000-0000-7000-8000-0000000000e1", kind: "message", method: "heuristic", strength: "moderate" },
          { evidence_id: "0192f000-0000-7000-8000-0000000000e2", kind: "message", method: "heuristic", strength: "moderate" },
        ],
      },
    ],
    notice: WORKING_HYPOTHESIS_NOTICE,
    third_party_data_is_local_only: true,
    ...overrides,
  };
}

/** A summary the way `soul-draft` builds one: counts, and what backs them. */
export function aPersonSummary(overrides: Partial<PersonSummary> = {}): PersonSummary {
  return {
    contact_id: "0192f000-0000-7000-8000-000000000002",
    source: "counts",
    text: "这个人：一共 6 次往来，其中你发出 3 次。工作假设，非临床结论",
    points: [
      {
        statement: "一共 6 次往来，分布在 4 天里",
        evidence_ids: [
          "0192f000-0000-7000-8000-0000000000e1",
          "0192f000-0000-7000-8000-0000000000e2",
        ],
        band: "moderate",
      },
    ],
    notice: WORKING_HYPOTHESIS_NOTICE,
    clinical_claim: false,
    ...overrides,
  };
}

/** What `prepare_draft` answers with: the shape of a request, never its text. */
export function anE1Plan(overrides: Partial<E1DraftPlan> = {}): E1DraftPlan {
  return {
    preparation_id: "0192f000-0000-7000-8000-0000000000f1",
    plan_hash: "d5".repeat(32),
    model: "unnamed-model",
    third_party_turns: 1,
    placeheld_turns: 1,
    carries_exempted_original: false,
    notice: E1_PLAN_NOTICE,
    not_sent_notice: NOT_SENT_NOTICE,
    ...overrides,
  };
}

/**
 * A draft the way the local path produces one.
 *
 * The counts are zero because that path builds no request body at all, which
 * is `soul-draft`'s `BodyFacts::default` and not a rounding of something.
 */
export function aTemplateDraft(overrides: Partial<Draft> = {}): Draft {
  return {
    text: "收到，我看一下，晚点回你。\n（这里写你要说的内容）",
    source: "tone_template",
    delivery: false,
    third_party_turns: 0,
    placeheld_turns: 0,
    carries_exempted_original: false,
    degraded: null,
    injection_signals: [],
    not_sent_notice: NOT_SENT_NOTICE,
    source_notice: TEMPLATE_NOTICE,
    ...overrides,
  };
}

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
/**
 * How the double answers the commands whose answer a test cares about.
 *
 * `drafting` is a function of the paste rather than a constant so a test can
 * assert on what the shell actually sent, and can throw to make the core
 * refuse the way a real refusal arrives — as a value, not an exception type.
 */
export interface FakeCoreOptions {
  readonly snapshot?: ConfigSnapshot;
  readonly status?: SessionStatus;
  readonly drafting?: (pasted: string) => Draft;
  readonly files?: FilesView;
  readonly authorizing?: (path: string) => FilesView;
  readonly planning?: (path: string) => PlanPreview;
  readonly graph?: PeopleGraph;
  /** As `graph`, but able to throw the way a refusal arrives — as a value. */
  readonly graphing?: () => PeopleGraph;
  readonly summarizing?: (contactId: string) => PersonSummary;
  readonly preparing?: (pasted: string) => E1DraftPlan;
  readonly generating?: (approval: { preparation_id: string; plan_hash: string }) => Draft;
}

export function installFakeCore(
  snapshotOrOptions: ConfigSnapshot | FakeCoreOptions = CLOSED_SNAPSHOT,
): FakeCore {
  const options: FakeCoreOptions =
    "collect_enabled" in snapshotOrOptions ? { snapshot: snapshotOrOptions } : snapshotOrOptions;
  const snapshot = options.snapshot ?? CLOSED_SNAPSHOT;
  const status = options.status ?? OPEN_SESSION;
  const drafting = options.drafting ?? (() => aTemplateDraft());
  const files = options.files ?? NO_ROOTS;
  const calls: RecordedCall[] = [];

  mockIPC((cmd, payload) => {
    calls.push({ cmd, payload });
    switch (cmd) {
      case "config_snapshot":
        return snapshot;
      case "session_status":
        return status;
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
      case "files_view":
        return files;
      case "authorize_directory":
        return (options.authorizing ?? (() => files))((payload as { path?: string }).path ?? "");
      case "preview_plan":
        return (options.planning ?? (() => aPlanPreview()))(
          (payload as { path?: string }).path ?? "",
        );
      case "people_graph":
        return (options.graphing ?? (() => options.graph ?? EMPTY_GRAPH))();
      case "person_summary":
        return (options.summarizing ?? (() => aPersonSummary()))(
          (payload as { contactId?: string }).contactId ?? "",
        );
      case "draft_notices":
        return DRAFT_NOTICES;
      case "draft_reply":
        return drafting((payload as { pasted?: string }).pasted ?? "");
      case "prepare_draft":
        return (options.preparing ?? (() => anE1Plan()))((payload as { pasted?: string }).pasted ?? "");
      case "generate_draft":
        return (options.generating ?? (() => aTemplateDraft({ source: "user_endpoint" })))(
          (payload as { approval: { preparation_id: string; plan_hash: string } }).approval,
        );
      case "discard_draft":
        return true;
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
