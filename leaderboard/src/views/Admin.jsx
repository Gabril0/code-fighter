import { useCallback, useEffect, useRef, useState } from "react";

import { api, setToken } from "../lib/api";
import { readImageAsSquareDataUrl } from "../lib/photo";

function CopyHash({ token }) {
  const [copied, setCopied] = useState(false);
  return (
    <button
      type="button"
      className="hash-chip"
      title="Click to copy"
      onClick={async () => {
        await navigator.clipboard.writeText(token);
        setCopied(true);
        setTimeout(() => setCopied(false), 1400);
      }}
    >
      {copied ? "Copied" : token}
    </button>
  );
}

function MemberPhoto({ user, onPatch, onError }) {
  const fileRef = useRef(null);
  const [busy, setBusy] = useState(false);

  async function pick(event) {
    const input = event.target;
    const file = input.files?.[0];
    if (!file) return;
    setBusy(true);
    try {
      const photo = await readImageAsSquareDataUrl(file);
      await onPatch(user.id, { photo });
    } catch (problem) {
      onError(`${user.name}: ${problem.message}`);
    } finally {
      setBusy(false);
      input.value = "";
    }
  }

  return (
    <div className="admin-photo">
      <button
        type="button"
        className="avatar-sm avatar-sm--editable"
        title={user.photo ? "Click to change" : "Click to upload"}
        onClick={() => fileRef.current?.click()}
      >
        {user.photo ? <img src={user.photo} alt="" /> : <span>{busy ? "…" : "+"}</span>}
      </button>
      <input ref={fileRef} type="file" accept="image/*" hidden onChange={pick} />
      {user.photo && (
        <button
          type="button"
          className="link danger-link photo-clear"
          onClick={() => onPatch(user.id, { photo: null })}
        >
          Remove
        </button>
      )}
    </div>
  );
}

function TeamCard({ team, onPatch, onDelete }) {
  return (
    <div className="admin-team" style={{ borderColor: team.color }}>
      <div className="admin-team-head">
        <div className="admin-team-fields">
          <input
            defaultValue={team.name}
            onBlur={(e) => e.target.value !== team.name && onPatch(team.id, { name: e.target.value })}
          />
          <div className="inline">
            <input
              type="color"
              className="color-input"
              defaultValue={team.color}
              onChange={(e) => onPatch(team.id, { color: e.target.value })}
            />
            <span className="hint">
              corner {team.side === "left" ? "left" : "right"} · {team.member_count} members · {team.score} pts
            </span>
          </div>
        </div>
      </div>
      <button type="button" className="link danger-link" onClick={() => onDelete(team.id)}>
        Delete team
      </button>
    </div>
  );
}

export default function Admin({ onSignedOut }) {
  const [state, setState] = useState(null);
  const [error, setError] = useState(null);
  const [newUser, setNewUser] = useState("");
  const [newTeam, setNewTeam] = useState("");

  const refresh = useCallback(async () => {
    try {
      setState(await api.admin.state());
      setError(null);
    } catch (problem) {
      setError(problem.message);
    }
  }, []);

  useEffect(() => {
    refresh();
    const timer = setInterval(refresh, 5000);
    return () => clearInterval(timer);
  }, [refresh]);

  const guard = (action) => async (...args) => {
    try {
      await action(...args);
      await refresh();
    } catch (problem) {
      setError(problem.message);
    }
  };

  if (error && !state) {
    return (
      <div className="sheet">
        <div className="card">
          <h1>Organizer</h1>
          <p className="status status--error">{error}</p>
          <a className="link" href="#/login">
            Sign in again
          </a>
        </div>
      </div>
    );
  }

  if (!state) return <div className="sheet"><p className="hint">Loading…</p></div>;

  const participants = state.users.filter((u) => u.role !== "admin");
  const unassigned = participants.filter((u) => !u.team_id);

  return (
    <div className="admin">
      <header className="admin-header">
        <h1>Organizer dashboard</h1>
        <div className="row-links">
          <span className={state.locked ? "pill pill--offline" : "pill"}>
            {state.locked ? "Submissions locked" : "Submissions open"}
          </span>
          <button
            type="button"
            className="link"
            onClick={guard(() => api.admin.setLock(!state.locked))}
          >
            {state.locked ? "Unlock" : "Lock"}
          </button>
          <a className="link" href="#/">
            Scoreboard
          </a>
          <button
            type="button"
            className="link"
            onClick={() => {
              setToken(null);
              onSignedOut();
            }}
          >
            Sign out
          </button>
        </div>
      </header>

      {error && <p className="status status--error">{error}</p>}

      <section className="admin-section">
        <h2>Championship</h2>
        <form
          className="inline"
          key={state.title ?? "Code Fighter"}
          onSubmit={(e) => {
            e.preventDefault();
            const value = e.target.elements.title.value.trim();
            if (value && value !== state.title) {
              guard(() => api.admin.setTitle(value))();
            }
          }}
        >
          <input
            name="title"
            defaultValue={state.title ?? "Code Fighter"}
            placeholder="Championship name"
            maxLength={60}
          />
          <button type="submit">Save name</button>
        </form>
        <p className="hint">Shown as the title on the big scoreboard screen.</p>
      </section>

      <section className="admin-section">
        <h2>Teams</h2>
        <div className="admin-teams">
          {state.teams.map((team) => (
            <TeamCard
              key={team.id}
              team={team}
              onPatch={guard((id, patch) => api.admin.patchTeam(id, patch))}
              onDelete={guard((id) =>
                confirm("Delete this team? Its scoreboard entries and members will be removed too.")
                  ? api.admin.deleteTeam(id)
                  : Promise.resolve(),
              )}
            />
          ))}
        </div>

        {state.teams.length < 2 && (
          <form
            className="inline"
            onSubmit={(e) => {
              e.preventDefault();
              if (newTeam.trim()) {
                guard(() => api.admin.createTeam(newTeam.trim()))();
                setNewTeam("");
              }
            }}
          >
            <input
              value={newTeam}
              onChange={(e) => setNewTeam(e.target.value)}
              placeholder="New team name"
            />
            <button type="submit">Create team</button>
          </form>
        )}
      </section>

      <section className="admin-section">
        <h2>People</h2>
        <form
          className="inline"
          onSubmit={(e) => {
            e.preventDefault();
            if (newUser.trim()) {
              guard(() => api.admin.createUser(newUser.trim()))();
              setNewUser("");
            }
          }}
        >
          <input
            value={newUser}
            onChange={(e) => setNewUser(e.target.value)}
            placeholder="Person name"
          />
          <button type="submit">Generate login</button>
        </form>

        {unassigned.length > 0 && (
          <p className="hint">
            {unassigned.length} still without a team.
          </p>
        )}

        <table className="admin-table">
          <thead>
            <tr>
              <th />
              <th>Name</th>
              <th>Login hash</th>
              <th>Team</th>
              <th>Profile</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {participants.map((user) => (
              <tr key={user.id}>
                <td>
                  <MemberPhoto
                    user={user}
                    onPatch={guard((id, patch) => api.admin.patchUser(id, patch))}
                    onError={setError}
                  />
                </td>
                <td>
                  <input
                    defaultValue={user.name}
                    onBlur={(e) =>
                      e.target.value !== user.name &&
                      guard(() => api.admin.patchUser(user.id, { name: e.target.value }))()
                    }
                  />
                </td>
                <td>
                  <CopyHash token={user.token} />
                </td>
                <td>
                  <select
                    value={user.team_id ?? ""}
                    onChange={(e) =>
                      guard(() =>
                        api.admin.patchUser(user.id, { team_id: e.target.value || null }),
                      )()
                    }
                  >
                    <option value="">No team</option>
                    {state.teams.map((team) => (
                      <option key={team.id} value={team.id}>
                        {team.name}
                      </option>
                    ))}
                  </select>
                </td>
                <td>
                  <span className={user.profile_set ? "tag tag--ok" : "tag"}>
                    {user.profile_set ? "READY" : "PENDING"}
                  </span>
                </td>
                <td>
                  <button
                    type="button"
                    className="link danger-link"
                    onClick={() =>
                      confirm(`Remove ${user.name}?`) &&
                      guard(() => api.admin.deleteUser(user.id))()
                    }
                  >
                    Remove
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>

      <section className="admin-section">
        <h2>Contest</h2>
        <p className="hint">
          {state.questions.length} questions ·{" "}
          {state.questions.reduce((sum, c) => sum + c.points, 0)} points total. Questions
          live in <code>server/questions/</code>.
        </p>
        <button
          type="button"
          className="danger"
          onClick={() =>
            confirm("Reset the contest? All solves and attempts will disappear.") &&
            guard(() => api.admin.reset())()
          }
        >
          Reset contest
        </button>
      </section>
    </div>
  );
}
