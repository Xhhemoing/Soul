/**
 * The shell: ask the core what this session is, show the wizard until it has
 * been through, then show whichever route the address bar names.
 *
 * There is no state here that means anything on its own. Every value on screen
 * came out of `core.ts`, and the only thing this component decides is which
 * component gets to render it. That now includes whether the wizard is behind
 * us: it used to be a prop, which meant a real installation asked the same
 * question on every launch. The answer lives beside the store, and the core
 * reads it.
 */

import { useEffect, useState } from "react";

import { NavRail } from "./components/NavRail";
import { Pending } from "./components/Pending";
import {
  configSnapshot,
  sessionStatus,
  startRuntimeEvents,
  subscribeDraftSource,
  hasPendingSourceChanges,
  getRuntimeSyncFailure,
  type ConfigSnapshot,
  type SessionStatus,
} from "./core";
import { Audit } from "./routes/Audit";
import { Collect } from "./routes/Collect";
import { Draft } from "./routes/Draft";
import { Files } from "./routes/Files";
import { Graph } from "./routes/Graph";
import { Home } from "./routes/Home";
import { Import } from "./routes/Import";
import { LocalInsights } from "./routes/LocalInsights";
import { Assistance } from "./routes/Assistance";
import { Memory } from "./routes/Memory";
import { Profile } from "./routes/Profile";
import { Research } from "./routes/Research";
import { Settings } from "./routes/Settings";
import { Wizard } from "./routes/Wizard";
import { useRoute } from "./router";

export function App(): React.JSX.Element {
  const [snapshot, setSnapshot] = useState<ConfigSnapshot | null>(null);
  const [status, setStatus] = useState<SessionStatus | null>(null);
  const [failure, setFailure] = useState<string | null>(null);
  /** Set when the wizard finishes, so this launch does not wait for a reread. */
  const [justFinished, setJustFinished] = useState(false);
  const route = useRoute();

  useEffect(() => startRuntimeEvents(), []);

  useEffect(() => {
    let live = true;
    let generation = 0;
    const read = (): void => {
      const ticket = ++generation;
      const syncFailure = getRuntimeSyncFailure();
      if (syncFailure !== null) { setFailure(syncFailure); return; }
      if (hasPendingSourceChanges()) return;
      void Promise.all([configSnapshot(), sessionStatus()]).then(
      ([snapshot, status]) => {
        if (!live || ticket !== generation) return;
        setSnapshot(snapshot);
        setStatus(status);
        setFailure(null);
      },
      (error: unknown) => {
        if (live && ticket === generation) setFailure(String(error));
      },
      );
    };
    // Runtime changes and a locally owned write's final settlement both wake
    // a deferred reread; recovering the runtime alone may still find it busy.
    const unsubscribe = subscribeDraftSource(read);
    read();
    return () => {
      live = false;
      unsubscribe();
    };
  }, []);

  if (failure !== null) {
    return (
      <main className="panel refusal" role="alert">
        <h1>核心没有回应</h1>
        <p>{failure}</p>
      </main>
    );
  }

  if (snapshot === null || status === null) {
    return (
      <main className="panel" aria-busy="true">
        <p>正在读取本机配置…</p>
      </main>
    );
  }

  if (!status.wizard_completed && !justFinished) {
    return (
      <Wizard
        snapshot={snapshot}
        onComplete={(completed) => {
          setSnapshot(completed);
          setJustFinished(true);
        }}
      />
    );
  }

  return (
    <div className="layout">
      <NavRail current={route} />
      <main className="content" aria-labelledby="route-title">
        <h1 id="route-title">{route.title}</h1>
        {route.id === "home" ? <Home snapshot={snapshot} status={status} /> : null}
        {route.id === "import" ? <Import /> : null}
        {route.id === "insights" ? <LocalInsights /> : null}
        {route.id === "assistance" ? <Assistance /> : null}
        {route.id === "collect" ? <Collect /> : null}
        {route.id === "profile" ? <Profile /> : null}
        {route.id === "files" ? <Files /> : null}
        {route.id === "graph" ? <Graph /> : null}
        {route.id === "memory" ? <Memory /> : null}
        {route.id === "draft" ? <Draft /> : null}
        {route.id === "research" ? <Research /> : null}
        {route.id === "audit" ? <Audit /> : null}
        {route.id === "settings" ? (
          <Settings snapshot={snapshot} onSnapshot={setSnapshot} />
        ) : null}
        {route.ownedBy === null ? null : (
          <Pending title={route.title} ownedBy={route.ownedBy} detail={route.pending ?? ""} />
        )}
      </main>
    </div>
  );
}
