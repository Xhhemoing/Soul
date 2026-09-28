/**
 * Real React/IPC regression cases. These use the repository's existing fake
 * core and Vitest setup; offline handler probes are not a substitute.
 */
import { StrictMode } from "react";
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { mockIPC } from "@tauri-apps/api/mocks";
import { describe, expect, it } from "vitest";

import type { PeopleGraph, PersonSummary } from "../core";
import { aPeopleGraph, aPersonSummary } from "../test/fakeCore";
import { Graph } from "./Graph";

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}

function fixture() {
  const graph = aPeopleGraph();
  const first = graph.people.find((person) => !person.is_you)!;
  const second = {
    ...first,
    contact_id: "0192f000-0000-7000-8000-000000000003",
    identifier_hint: "cccccccc",
  };
  return {
    graph: { ...graph, people: [...graph.people, second] },
    firstId: first.contact_id,
    secondId: second.contact_id,
    tieId: graph.ties[0]!.relationship_id,
  };
}

async function peopleButtons() {
  return screen.findAllByRole("button", { name: "看这个人的摘要" });
}

describe("graph lifecycle across IPC responses", () => {
  it("does not display a response for a different contact", async () => {
    const { graph, secondId } = fixture();
    mockIPC((command) => {
      if (command === "people_graph") return graph;
      if (command === "person_summary") {
        return aPersonSummary({ contact_id: secondId, text: "WRONG_CONTACT_CONTENT" });
      }
      throw new Error(`unexpected command: ${command}`);
    });
    const user = userEvent.setup();
    render(<Graph />);
    await user.click((await peopleButtons())[0]!);
    expect(await screen.findByTestId("summary-refusal-code")).toHaveTextContent("ROUTINE");
    expect(screen.queryByText("WRONG_CONTACT_CONTENT")).toBeNull();
    expect(screen.queryByTestId("summary-text")).toBeNull();
    expect((await peopleButtons())[0]).toBeEnabled();
  });

  it("disables only the pending contact and allows selecting another", async () => {
    const { graph, firstId, secondId } = fixture();
    const pending = deferred<PersonSummary>();
    let calls = 0;
    mockIPC((command) => {
      if (command === "people_graph") return graph;
      if (command === "person_summary") {
        calls += 1;
        return calls === 1 ? pending.promise
          : aPersonSummary({ contact_id: secondId, text: "SECOND_CONTACT" });
      }
      throw new Error(`unexpected command: ${command}`);
    });
    const user = userEvent.setup();
    render(<Graph />);
    const buttons = await peopleButtons();
    await user.click(buttons[0]!);
    expect(buttons[0]).toBeDisabled();
    expect(buttons[1]).toBeEnabled();
    await user.click(buttons[0]!);
    expect(calls).toBe(1);
    await user.click(buttons[1]!);
    expect(await screen.findByTestId("summary-text")).toHaveTextContent("SECOND_CONTACT");
    await act(async () => pending.resolve(aPersonSummary({ contact_id: firstId, text: "OLD" })));
    expect(screen.getByTestId("summary-text")).toHaveTextContent("SECOND_CONTACT");
  });

  it("allows retry after a rejected summary", async () => {
    const { graph, firstId } = fixture();
    let calls = 0;
    mockIPC((command) => {
      if (command === "people_graph") return graph;
      if (command === "person_summary") {
        calls += 1;
        if (calls === 1) return Promise.reject({ reason_code: "ROUTINE", explanation: "try again" });
        return aPersonSummary({ contact_id: firstId, text: "RETRIED" });
      }
      throw new Error(`unexpected command: ${command}`);
    });
    const user = userEvent.setup();
    render(<Graph />);
    await user.click((await peopleButtons())[0]!);
    await screen.findByTestId("summary-refusal-code");
    await user.click((await peopleButtons())[0]!);
    expect(await screen.findByTestId("summary-text")).toHaveTextContent("RETRIED");
    expect(screen.queryByTestId("summary-refusal-code")).toBeNull();
    expect(calls).toBe(2);
  });

  it("admits at most one correction before the pending write settles", async () => {
    const { graph, tieId } = fixture();
    const pending = deferred<PeopleGraph>();
    let calls = 0;
    mockIPC((command) => {
      if (command === "people_graph") return graph;
      if (command === "correct_tie") { calls += 1; return pending.promise; }
      throw new Error(`unexpected command: ${command}`);
    });
    render(<Graph />);
    await peopleButtons();
    const tie = within(screen.getByTestId(`tie-${tieId}`));
    const weak = tie.getByRole("button", { name: "弱" });
    const strong = tie.getByRole("button", { name: "强" });
    act(() => { fireEvent.click(weak); fireEvent.click(strong); });
    expect(calls).toBe(1);
    for (const button of await peopleButtons()) expect(button).toBeDisabled();
    await act(async () => pending.resolve(graph));
    await waitFor(() => expect(weak).toBeEnabled());
  });

  it("unblocks graph controls after a write rejection", async () => {
    const { graph, tieId } = fixture();
    let calls = 0;
    mockIPC((command) => {
      if (command === "people_graph") return graph;
      if (command === "correct_tie") {
        calls += 1;
        if (calls === 1) return Promise.reject({ reason_code: "ROUTINE", explanation: "write failed" });
        return graph;
      }
      throw new Error(`unexpected command: ${command}`);
    });
    const user = userEvent.setup();
    render(<Graph />);
    await peopleButtons();
    const weak = within(screen.getByTestId(`tie-${tieId}`)).getByRole("button", { name: "弱" });
    await user.click(weak);
    await screen.findByTestId("tie-refusal-code");
    expect(weak).toBeEnabled();
    await user.click(weak);
    await waitFor(() => expect(screen.queryByTestId("tie-refusal-code")).toBeNull());
    expect(calls).toBe(2);
  });

  it.each(["success", "failure"] as const)(
    "ignores an older summary %s after a correction",
    async (outcome) => {
      const { graph, firstId, tieId } = fixture();
      const pending = deferred<PersonSummary>();
      mockIPC((command) => {
        if (command === "people_graph" || command === "correct_tie") return graph;
        if (command === "person_summary") return pending.promise;
        throw new Error(`unexpected command: ${command}`);
      });
      const user = userEvent.setup();
      render(<Graph />);
      await user.click((await peopleButtons())[0]!);
      await user.click(within(screen.getByTestId(`tie-${tieId}`)).getByRole("button", { name: "弱" }));
      await act(async () => {
        if (outcome === "success") pending.resolve(aPersonSummary({ contact_id: firstId, text: "STALE" }));
        else pending.reject({ reason_code: "ROUTINE", explanation: "stale failure" });
      });
      expect(screen.queryByTestId("summary-text")).toBeNull();
      expect(screen.queryByTestId("summary-refusal-code")).toBeNull();
    },
  );

  it("rejects the older response after selecting A, B, then A again", async () => {
    const { graph, firstId, secondId } = fixture();
    const pending = [deferred<PersonSummary>(), deferred<PersonSummary>(), deferred<PersonSummary>()];
    let calls = 0;
    mockIPC((command) => {
      if (command === "people_graph") return graph;
      if (command === "person_summary") return pending[calls++]!.promise;
      throw new Error(`unexpected command: ${command}`);
    });
    const user = userEvent.setup();
    render(<Graph />);
    const buttons = await peopleButtons();
    await user.click(buttons[0]!);
    await user.click(buttons[1]!);
    await user.click(buttons[0]!);
    await act(async () => {
      pending[2]!.resolve(aPersonSummary({ contact_id: firstId, text: "CURRENT_A" }));
    });
    await act(async () => {
      pending[0]!.resolve(aPersonSummary({ contact_id: firstId, text: "OLD_A" }));
      pending[1]!.resolve(aPersonSummary({ contact_id: secondId, text: "OLD_B" }));
    });
    expect(screen.getByTestId("summary-text")).toHaveTextContent("CURRENT_A");
  });

  it.each(["success", "failure"] as const)(
    "does not carry an unmounted write %s into a fresh page",
    async (outcome) => {
      const { graph, firstId, tieId } = fixture();
      const pending = deferred<PeopleGraph>();
      mockIPC((command) => {
        if (command === "people_graph") return graph;
        if (command === "correct_tie") return pending.promise;
        if (command === "person_summary") return aPersonSummary({ contact_id: firstId, text: "FRESH" });
        throw new Error(`unexpected command: ${command}`);
      });
      const user = userEvent.setup();
      const previous = render(<Graph />);
      await peopleButtons();
      await user.click(within(screen.getByTestId(`tie-${tieId}`)).getByRole("button", { name: "弱" }));
      previous.unmount();
      render(<Graph />);
      await user.click((await peopleButtons())[0]!);
      expect(await screen.findByTestId("summary-text")).toHaveTextContent("FRESH");
      await act(async () => {
        if (outcome === "success") pending.resolve(graph);
        else pending.reject({ reason_code: "ROUTINE", explanation: "old write" });
      });
      expect(screen.getByTestId("summary-text")).toHaveTextContent("FRESH");
      expect(screen.queryByTestId("tie-refusal-code")).toBeNull();
    },
  );

  it("accepts requests after StrictMode replays the effect lifecycle", async () => {
    const { graph, firstId } = fixture();
    mockIPC((command) => {
      if (command === "people_graph") return graph;
      if (command === "person_summary") return aPersonSummary({ contact_id: firstId, text: "STRICT_MODE" });
      throw new Error(`unexpected command: ${command}`);
    });
    const user = userEvent.setup();
    render(<StrictMode><Graph /></StrictMode>);
    await user.click((await peopleButtons())[0]!);
    expect(await screen.findByTestId("summary-text")).toHaveTextContent("STRICT_MODE");
  });
});