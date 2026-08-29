import { useState } from "react";

import { api, setToken } from "../lib/api";

export default function Login({ onSignedIn }) {
  const [hash, setHash] = useState("");
  const [error, setError] = useState(null);
  const [busy, setBusy] = useState(false);

  async function handleSubmit(event) {
    event.preventDefault();
    const token = hash.trim();
    if (!token) return setError("Paste the hash you received");

    setBusy(true);
    setError(null);
    try {
      const user = await api.authenticate(token);
      setToken(token);
      onSignedIn(user);
    } catch (problem) {
      setError(problem.message);
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="sheet">
      <form className="card" onSubmit={handleSubmit}>
        <h1>Code Fighter</h1>
        <p className="hint">Paste the login hash the organizer gave you.</p>

        <input
          className="hash-input"
          value={hash}
          onChange={(e) => setHash(e.target.value)}
          placeholder="Your login hash"
          autoComplete="off"
          spellCheck={false}
          autoFocus
        />

        <button type="submit" disabled={busy}>
          {busy ? "Checking…" : "Sign in"}
        </button>

        {error && <p className="status status--error">{error}</p>}

        <a className="link" href="#/">
          I just want to see the scoreboard
        </a>
      </form>
    </div>
  );
}
