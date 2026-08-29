import { useState } from "react";

import { api, setSpectator, setToken } from "../lib/api";
import { readImageAsSquareDataUrl } from "../lib/photo";

export default function Join({ onSignedIn, onSpectating }) {
  const [hash, setHash] = useState("");
  const [name, setName] = useState("");
  const [photo, setPhoto] = useState(null);
  const [error, setError] = useState(null);
  const [busy, setBusy] = useState(false);

  const signIn = async (event) => {
    event.preventDefault();
    setBusy(true);
    setError(null);
    try {
      const trimmed = hash.trim();
      const member = await api.authenticate(trimmed);
      setToken(trimmed);
      setSpectator(null);
      onSignedIn(member);
    } catch (problem) {
      setError(problem.message);
    } finally {
      setBusy(false);
    }
  };

  const watch = async (event) => {
    event.preventDefault();
    setBusy(true);
    setError(null);
    try {
      const guest = await api.spectate({ name: name.trim(), photo });
      setToken(guest.token);
      setSpectator(guest);
      onSpectating(guest);
    } catch (problem) {
      setError(problem.message);
    } finally {
      setBusy(false);
    }
  };

  const pickPhoto = async (event) => {
    const file = event.target.files?.[0];
    if (!file) return;
    try {
      setPhoto(await readImageAsSquareDataUrl(file));
    } catch (problem) {
      setError(problem.message);
    }
  };

  return (
    <div className="sheet">
      <div className="join">
        <form className="card" onSubmit={signIn}>
          <h1>Competitor</h1>
          <p className="hint">Have an organizer hash? Step into the ring.</p>
          <input
            className="hash-input"
            value={hash}
            onChange={(e) => setHash(e.target.value)}
            placeholder="Your login hash"
            autoComplete="off"
          />
          <button type="submit" disabled={busy || !hash.trim()}>
            Enter the ring
          </button>
        </form>

        <form className="card" onSubmit={watch}>
          <h1>Crowd</h1>
          <p className="hint">No hash? Grab a seat, chat, and cheer from the stands.</p>
          <label>
            Seat name
            <input
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="How you appear in the stands"
              maxLength={22}
              autoComplete="off"
            />
          </label>
          <label className="join-photo">
            <span>Photo</span>
            <input type="file" accept="image/*" onChange={pickPhoto} />
          </label>
          {photo && <img className="join-preview" src={photo} alt="" />}
          <button type="submit" disabled={busy || !name.trim()}>
            Sit in the stands
          </button>
        </form>
      </div>
      {error && <p className="status status--error">{error}</p>}
    </div>
  );
}
