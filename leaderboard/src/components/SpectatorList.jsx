import { useState } from "react";

import { initials } from "../lib/initials";

function CrowdIcon() {
  return (
    <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true">
      <circle cx="9" cy="8" r="3.2" fill="none" stroke="currentColor" strokeWidth="2" />
      <path
        d="M3.5 19a5.5 5.5 0 0 1 11 0"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
      />
      <path
        d="M16 5.5a3.2 3.2 0 0 1 0 5M17.5 19a5.5 5.5 0 0 0-2.2-4.4"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
      />
    </svg>
  );
}

export default function SpectatorList({ people, viewerId }) {
  const [open, setOpen] = useState(false);
  const crowd = people ?? [];

  return (
    <div className="crowd-list">
      <button
        type="button"
        className={open ? "crowd-list-button crowd-list-button--on" : "crowd-list-button"}
        onClick={() => setOpen((value) => !value)}
        title="Who is watching"
      >
        <CrowdIcon />
        <span>Crowd</span>
        <em>{crowd.length}</em>
      </button>

      {open && (
        <section className="crowd-list-panel">
          {crowd.length === 0 ? (
            <p className="hint">The stands are empty for now.</p>
          ) : (
            crowd.map((person) => (
              <div
                key={person.id}
                className={
                  person.id === viewerId ? "crowd-person crowd-person--you" : "crowd-person"
                }
              >
                <div className="crowd-person-photo">
                  {person.photo ? <img src={person.photo} alt="" /> : null}
                  <span>{initials(person.name)}</span>
                </div>
                <span className="crowd-person-name">{person.name}</span>
                {person.id === viewerId && <em className="crowd-person-tag">You</em>}
              </div>
            ))
          )}
        </section>
      )}
    </div>
  );
}
