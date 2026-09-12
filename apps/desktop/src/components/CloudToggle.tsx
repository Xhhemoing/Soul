/**
 * AC-22: the cloud switch is visible, it reads 尚未启用, and pressing it does
 * nothing except say so again.
 *
 * The component holds no opinion about what the cloud is. It asks the core and
 * renders the answer, which is why pressing the switch cannot turn anything on
 * even if someone edits this file: there is no state here to flip.
 */

import { useState } from "react";

import { cloudToggle, type CloudNotice } from "../core";

export interface CloudToggleProps {
  readonly notice: CloudNotice;
}

export function CloudToggle({ notice }: CloudToggleProps): React.JSX.Element {
  const [current, setCurrent] = useState<CloudNotice>(notice);
  const [pressed, setPressed] = useState(false);

  const onPress = async (): Promise<void> => {
    // Ask for the opposite of what the switch shows. The core returns the same
    // notice either way; asking for "on" is what makes that a claim and not an
    // assumption.
    const answer = await cloudToggle(!current.enabled);
    setCurrent(answer);
    setPressed(true);
  };

  return (
    <section className="panel" aria-labelledby="cloud-heading">
      <h2 id="cloud-heading">云端深度分析</h2>
      <div className="switch-row">
        <button
          type="button"
          role="switch"
          aria-checked={current.enabled}
          className="switch"
          onClick={() => {
            void onPress();
          }}
        >
          <span className="switch-track" aria-hidden="true" />
          <span className="switch-label">{current.label}</span>
        </button>
        <span className="badge" data-testid="cloud-state">
          {current.label}
        </span>
      </div>
      <p className="muted">{current.explanation}</p>
      {pressed ? (
        <p className="muted" role="status" data-testid="cloud-press-result">
          你刚才点了这个开关。它仍然是「{current.label}」，本机没有发出任何请求。
        </p>
      ) : null}
    </section>
  );
}
