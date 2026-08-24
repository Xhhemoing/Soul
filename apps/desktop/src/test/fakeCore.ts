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

import type { CloudNotice, ConfigSnapshot } from "../core";

export const CLOUD_LABEL = "尚未启用";

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
export function installFakeCore(snapshot: ConfigSnapshot = CLOSED_SNAPSHOT): FakeCore {
  const calls: RecordedCall[] = [];

  mockIPC((cmd, payload) => {
    calls.push({ cmd, payload });
    switch (cmd) {
      case "config_snapshot":
        return snapshot;
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
