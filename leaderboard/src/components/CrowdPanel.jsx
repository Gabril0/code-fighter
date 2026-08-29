import { useEffect, useRef, useState } from "react";

import { api } from "../lib/api";

const EMOTES = [
  { id: "wave", label: "Wave", icon: "👋" },
  { id: "laugh", label: "Laugh", icon: "😂" },
  { id: "dance", label: "Dance", icon: "🕺" },
];

const CHAT_LIFETIME_MS = 40000;
const CHAT_FADE_MS = 4000;

function ChatIcon() {
  return (
    <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true">
      <path d="M4 5h16v10H8l-4 4V5z" fill="none" stroke="currentColor" strokeWidth="2" strokeLinejoin="round" />
    </svg>
  );
}

function EmoteIcon() {
  return (
    <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true">
      <circle cx="12" cy="12" r="8.5" fill="none" stroke="currentColor" strokeWidth="2" />
      <circle cx="9" cy="10" r="1.3" fill="currentColor" />
      <circle cx="15" cy="10" r="1.3" fill="currentColor" />
      <path d="M8.5 14.5a4.5 4.5 0 0 0 7 0" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
    </svg>
  );
}

function LeaveIcon() {
  return (
    <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true">
      <path d="M10 4H5v16h5" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
      <path
        d="M14 8l4 4-4 4M18 12H9"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

export default function CrowdPanel({ chat, canEmote, onLeaveSeat }) {
  const [panel, setPanel] = useState(null);
  const [draft, setDraft] = useState("");
  const [error, setError] = useState(null);
  const [sending, setSending] = useState(false);
  const [unread, setUnread] = useState(0);
  const [now, setNow] = useState(() => Date.now());
  const logRef = useRef(null);
  const seenRef = useRef(new Set());
  const arrivalRef = useRef(new Map());

  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 500);
    return () => clearInterval(timer);
  }, []);

  const arrivals = arrivalRef.current;
  const live = chat.filter((entry) => {
    const shownAt = arrivals.get(entry.id) ?? now;
    return now - shownAt < CHAT_LIFETIME_MS;
  });
  const liveKey = live.map((entry) => entry.id).join(",");

  useEffect(() => {
    const at = Date.now();
    const ids = new Set(chat.map((entry) => entry.id));
    chat.forEach((entry) => {
      if (!arrivals.has(entry.id)) arrivals.set(entry.id, at);
    });
    arrivals.forEach((_, id) => {
      if (!ids.has(id)) arrivals.delete(id);
    });
  }, [chat, arrivals]);

  useEffect(() => {
    if (panel === "chat") {
      live.forEach((entry) => seenRef.current.add(entry.id));
      setUnread(0);
      const log = logRef.current;
      if (log) log.scrollTop = log.scrollHeight;
      return;
    }
    setUnread(live.filter((entry) => !seenRef.current.has(entry.id)).length);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [liveKey, panel]);

  const toggle = (name) => setPanel((current) => (current === name ? null : name));

  const send = async (event) => {
    event.preventDefault();
    const body = draft.trim();
    if (!body) return;
    setSending(true);
    setError(null);
    try {
      await api.say(body);
      setDraft("");
    } catch (problem) {
      setError(problem.message);
    } finally {
      setSending(false);
    }
  };

  const react = async (emote) => {
    setError(null);
    try {
      await api.emote(emote);
    } catch (problem) {
      setError(problem.message);
    }
  };

  return (
    <div className="dock">
      {panel === "chat" && (
        <section className="dock-panel">
          <h2>Crowd chat</h2>
          <div className="crowd-log" ref={logRef}>
            {live.length === 0 && <p className="hint">No one has said anything yet.</p>}
            {live.map((entry) => (
              <p
                key={entry.id}
                className={[
                  "crowd-line",
                  entry.authorKind === "member" ? "crowd-line--member" : "",
                  now - (arrivals.get(entry.id) ?? now) > CHAT_LIFETIME_MS - CHAT_FADE_MS
                    ? "crowd-line--fading"
                    : "",
                ]
                  .filter(Boolean)
                  .join(" ")}
              >
                <strong style={entry.color ? { color: entry.color } : undefined}>
                  {entry.author}
                </strong>
                <span>{entry.body}</span>
              </p>
            ))}
          </div>
          <form className="inline" onSubmit={send}>
            <input
              value={draft}
              onChange={(e) => setDraft(e.target.value)}
              placeholder="Say something…"
              maxLength={160}
            />
            <button type="submit" disabled={sending || !draft.trim()}>
              Send
            </button>
          </form>
          {error && <p className="status status--error">{error}</p>}
        </section>
      )}

      {panel === "emotes" && (
        <section className="dock-panel">
          <h2>Reactions</h2>
          <div className="dock-emotes">
            {EMOTES.map((emote) => (
              <button key={emote.id} type="button" className="dock-emote" onClick={() => react(emote.id)}>
                <span aria-hidden="true">{emote.icon}</span>
                {emote.label}
              </button>
            ))}
          </div>
          {error && <p className="status status--error">{error}</p>}
        </section>
      )}

      <div className="dock-bar">
        <button
          type="button"
          className={panel === "chat" ? "dock-button dock-button--on" : "dock-button"}
          onClick={() => toggle("chat")}
          title="Crowd chat"
        >
          <ChatIcon />
          <span>Chat</span>
          {unread > 0 && panel !== "chat" && <em className="dock-badge">{unread}</em>}
        </button>

        {canEmote && (
          <button
            type="button"
            className={panel === "emotes" ? "dock-button dock-button--on" : "dock-button"}
            onClick={() => toggle("emotes")}
            title="Reactions"
          >
            <EmoteIcon />
            <span>Reactions</span>
          </button>
        )}

        {onLeaveSeat && (
          <button type="button" className="dock-button" onClick={onLeaveSeat} title="Leave seat">
            <LeaveIcon />
            <span>Exit</span>
          </button>
        )}
      </div>
    </div>
  );
}
