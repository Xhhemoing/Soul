/**
 * The shell: fetch the configuration once, show the wizard until it is done,
 * then show whichever route the address bar names.
 *
 * There is no state here that means anything on its own. Every value on screen
 * came out of `core.ts`, and the only thing this component decides is which
 * component gets to render it.
 */

import { useEffect, useState } from "react";

import { NavRail } from "./components/NavRail";
import { Pending } from "./components/Pending";
import { configSnapshot, type ConfigSnapshot } from "./core";
import { Home } from "./routes/Home";
import { Settings } from "./routes/Settings";
import { Wizard } from "./routes/Wizard";
import { useRoute } from "./router";

export interface AppProps {
  /**
   * Skip the first-run wizard. Only the tests for other routes pass this;
   * whether a real installation has finished the wizard is a configuration
   * question, and the file that answers it belongs to WP13.
   */
  readonly wizardDone?: boolean;
}

export function App({ wizardDone = false }: AppProps): React.JSX.Element {
  const [snapshot, setSnapshot] = useState<ConfigSnapshot | null>(null);
  const [failure, setFailure] = useState<string | null>(null);
  const [finished, setFinished] = useState(wizardDone);
  const route = useRoute();

  useEffect(() => {
    let live = true;
    configSnapshot().then(
      (value) => {
        if (live) setSnapshot(value);
      },
      (error: unknown) => {
        if (live) setFailure(String(error));
      },
    );
    return () => {
      live = false;
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

  if (snapshot === null) {
    return (
      <main className="panel" aria-busy="true">
        <p>正在读取本机配置…</p>
      </main>
    );
  }

  if (!finished) {
    return (
      <Wizard
        snapshot={snapshot}
        onComplete={(completed) => {
          setSnapshot(completed);
          setFinished(true);
        }}
      />
    );
  }

  return (
    <div className="layout">
      <NavRail current={route} />
      <main className="content" aria-labelledby="route-title">
        <h1 id="route-title">{route.title}</h1>
        {route.id === "home" ? <Home snapshot={snapshot} /> : null}
        {route.id === "settings" ? <Settings snapshot={snapshot} /> : null}
        {route.ownedBy === null ? null : (
          <Pending title={route.title} ownedBy={route.ownedBy} detail={route.pending ?? ""} />
        )}
      </main>
    </div>
  );
}
