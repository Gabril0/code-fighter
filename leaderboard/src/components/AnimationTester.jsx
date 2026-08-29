import { useState } from "react";

import { api } from "../lib/api";

const CLIPS = {
  attacks: ["Punch", "Kick", "DoublePunch", "Headbutt", "Tackle", "Shoryuken", "Haymaker"],
  reactions: ["Hit", "Clash", "Death", "Walking", "Sitting"],
  ringside: ["Cheer", "Dance", "Dance2", "Standing", "Idle", "ShadowBox", "Anxious", "LookAround", "Scream", "ArmWave", "Still"],
  effects: ["hit", "punch", "knockout", "victory", "solve"],
};

export default function AnimationTester({ arenaRef }) {
  const [side, setSide] = useState("left");
  const [last, setLast] = useState(null);
  const [open, setOpen] = useState(false);

  const run = (label, action) => {
    action();
    setLast(label);
  };

  const clipButtons = (names) =>
    names.map((clip) => (
      <button
        key={clip}
        type="button"
        className="tester-chip"
        onClick={() => run(clip, () => arenaRef.current?.previewClip(side, clip))}
      >
        {clip}
      </button>
    ));

  if (!open) {
    return (
      <aside className="tester tester--closed">
        <button type="button" className="tester-chip" onClick={() => setOpen(true)}>
          Animation tester ▸
        </button>
      </aside>
    );
  }

  return (
    <aside className="tester">
      <div className="tester-head">
        <button type="button" className="tester-toggle" onClick={() => setOpen(false)}>
          Animation tester ▾
        </button>
        <div className="tester-sides">
          {["left", "right"].map((value) => (
            <button
              key={value}
              type="button"
              className={side === value ? "tester-chip tester-chip--on" : "tester-chip"}
              onClick={() => setSide(value)}
            >
              {value === "left" ? "Left" : "Right"}
            </button>
          ))}
        </div>
      </div>

      <span className="tag">Attacks</span>
      <div className="tester-grid">{clipButtons(CLIPS.attacks)}</div>

      <span className="tag">Reactions</span>
      <div className="tester-grid">{clipButtons(CLIPS.reactions)}</div>

      <span className="tag">Ringside</span>
      <div className="tester-grid">{clipButtons(CLIPS.ringside)}</div>

      <span className="tag">Effects</span>
      <div className="tester-grid">
        {CLIPS.effects.map((kind) => (
          <button
            key={kind}
            type="button"
            className="tester-chip"
            onClick={() => run(`fx:${kind}`, () => arenaRef.current?.burst(kind, side))}
          >
            {kind}
          </button>
        ))}
        <button
          type="button"
          className="tester-chip tester-chip--ghost"
          onClick={() => run("ghost (all)", () => api.summonGhost().catch(() => {}))}
        >
          Ghost
        </button>
      </div>

      {last && <p className="hint">Ran: {last}</p>}
    </aside>
  );
}
