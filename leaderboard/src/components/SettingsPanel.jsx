import { useState } from "react";

import { api } from "../lib/api";

const SWITCHES = [
  { key: "pixelate", label: "Pixelation" },
  { key: "crt", label: "CRT effect" },
  { key: "aberration", label: "Chromatic aberration" },
  { key: "bloom", label: "Glow (bloom)" },
];

function GearIcon() {
  return (
    <svg width="15" height="15" viewBox="0 0 24 24" aria-hidden="true">
      <circle cx="12" cy="12" r="3" fill="none" stroke="currentColor" strokeWidth="2" />
      <path
        d="M12 2v3M12 19v3M2 12h3M19 12h3M4.9 4.9l2.2 2.2M16.9 16.9l2.2 2.2M19.1 4.9l-2.2 2.2M7.1 16.9l-2.2 2.2"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
      />
    </svg>
  );
}

export default function SettingsPanel({ effects, onChange, canSave }) {
  const [open, setOpen] = useState(false);
  const [error, setError] = useState(null);

  async function toggle(key) {
    const next = { ...effects, [key]: !effects[key] };
    onChange(next);
    if (!canSave) return;
    try {
      await api.saveEffects(next);
      setError(null);
    } catch (problem) {
      setError(problem.message);
    }
  }

  return (
    <div className="settings">
      {open && (
        <div className="settings-panel">
          <h2>Visual effects</h2>
          {SWITCHES.map((item) => (
            <label key={item.key} className="settings-row">
              {item.label}
              <input
                type="checkbox"
                checked={Boolean(effects[item.key])}
                onChange={() => toggle(item.key)}
              />
            </label>
          ))}
          <p className="settings-note">
            {canSave
              ? "Saved to your login and applies on every screen."
              : "Sign in to save this setting to your login."}
          </p>
          {error && <p className="status status--error">{error}</p>}
        </div>
      )}

      <button
        type="button"
        className={open ? "desk-button desk-button--on" : "desk-button"}
        onClick={() => setOpen((v) => !v)}
        title="Visual effects"
      >
        <GearIcon />
        Effects
      </button>
    </div>
  );
}
