import { useRef, useState } from "react";

import { api, setToken } from "../lib/api";
import { readImageAsSquareDataUrl } from "../lib/photo";

export default function Profile({ user, onUpdated, onSignedOut }) {
  const [name, setName] = useState(user.name ?? "");
  const [photo, setPhoto] = useState(user.photo ?? null);
  const [status, setStatus] = useState(null);
  const [busy, setBusy] = useState(false);
  const fileRef = useRef(null);

  async function pickPhoto(event) {
    const file = event.target.files?.[0];
    if (!file) return;
    try {
      setPhoto(await readImageAsSquareDataUrl(file));
      setStatus(null);
    } catch (problem) {
      setStatus({ kind: "error", text: problem.message });
    }
  }

  async function handleSubmit(event) {
    event.preventDefault();
    if (!name.trim()) return setStatus({ kind: "error", text: "Name cannot be empty" });

    setBusy(true);
    try {
      const updated = await api.updateMe({ name: name.trim(), photo });
      onUpdated(updated);
      setStatus({ kind: "ok", text: "Saved, you are in the ring" });
    } catch (problem) {
      setStatus({ kind: "error", text: problem.message });
    } finally {
      setBusy(false);
    }
  }

  function signOut() {
    setToken(null);
    onSignedOut();
  }

  return (
    <div className="sheet">
      <form className="card" onSubmit={handleSubmit}>
        <h1>Your competitor</h1>
        <p className="hint">
          This is the name and photo shown on the big screen and on your fighter's face.
        </p>

        <button
          type="button"
          className="photo-picker"
          onClick={() => fileRef.current?.click()}
          style={{ borderColor: user.team?.color ?? "#34d399" }}
        >
          {photo ? <img src={photo} alt="" /> : <span>Add a photo</span>}
        </button>
        <input
          ref={fileRef}
          type="file"
          accept="image/*"
          hidden
          onChange={pickPhoto}
        />

        <label>
          Name
          <input value={name} onChange={(e) => setName(e.target.value)} autoComplete="off" />
        </label>

        <p className="hint">
          Team:{" "}
          {user.team ? (
            <strong style={{ color: user.team.color }}>{user.team.name}</strong>
          ) : (
            "not on a team yet; the organizer will add you to one"
          )}
        </p>

        <button type="submit" disabled={busy}>
          {busy ? "Saving…" : "Save"}
        </button>

        {status && <p className={`status status--${status.kind}`}>{status.text}</p>}

        <div className="row-links">
          <a className="link" href="#/">
            Scoreboard
          </a>
          {user.role === "admin" && (
            <a className="link" href="#/admin">
              Organizer
            </a>
          )}
          <button type="button" className="link" onClick={signOut}>
            Sign out
          </button>
        </div>
      </form>
    </div>
  );
}
