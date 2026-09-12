import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import Arena from "../components/Arena";
import { useStageScale } from "../lib/stage";
import AnimationTester from "../components/AnimationTester";
import CrowdPanel from "../components/CrowdPanel";
import Scoreboard from "../components/Scoreboard";
import SettingsPanel from "../components/SettingsPanel";
import SpectatorList from "../components/SpectatorList";
import SubmitPanel from "../components/SubmitPanel";
import TeamHud from "../components/TeamHud";
import { api } from "../lib/api";

const POLL_MS = 2500;
const LIVE_MS = 1500;
const TOAST_MS = 6000;
const HEARTBEAT_MS = 30000;

export default function Board({ user, spectator, onLeaveSeat }) {
  const arenaRef = useRef(null);
  const frameRef = useStageScale();
  const previousRef = useRef(null);
  const sidesRef = useRef({});
  const [teams, setTeams] = useState([]);
  const [questions, setQuestions] = useState([]);
  const [connection, setConnection] = useState({ status: "connecting" });
  const [locked, setLocked] = useState(false);
  const [title, setTitle] = useState("Code Fighter");
  const [toasts, setToasts] = useState([]);
  const [chat, setChat] = useState([]);
  const [spectators, setSpectators] = useState([]);
  const cursorRef = useRef(0);
  const [focusMode, setFocusMode] = useState(false);
  const [effects, setEffects] = useState(
    () => user?.effects ?? spectator?.effects ?? { pixelate: true, crt: true, aberration: true, bloom: true },
  );

  const pushToast = useCallback((toast) => {
    const key = `${toast.team}-${toast.question}-${Date.now()}`;
    setToasts((current) => [{ ...toast, key }, ...current].slice(0, 3));
    setTimeout(() => setToasts((current) => current.filter((t) => t.key !== key)), TOAST_MS);
  }, []);

  useEffect(() => {
    document.title = `${title} — Scoreboard`;
  }, [title]);

  useEffect(() => {
    api
      .questions()
      .then(setQuestions)
      .catch(() => {});
  }, []);

  useEffect(() => {
    let cancelled = false;

    const tick = async () => {
      try {
        const board = await api.board();
        if (cancelled) return;
        setLocked(board.locked);
        if (board.title) setTitle(board.title);
        setConnection({ status: "online" });
        sidesRef.current = Object.fromEntries(board.teams.map((team) => [team.id, team.side]));

        const previous = previousRef.current;
        if (previous) {
          for (const team of board.teams) {
            const before = previous.find((t) => t.id === team.id);
            const gained = Object.keys(team.solves).filter(
              (id) => !before || !(id in before.solves),
            );
            for (const questionId of gained) {
              arenaRef.current?.punch(team.side);
              const question = questions.find((c) => c.id === questionId);
              pushToast({
                team: team.name,
                question: question?.title ?? questionId,
                points: team.solves[questionId].points,
                color: team.color,
              });
            }
          }
        }
        previousRef.current = board.teams;
        setTeams(board.teams);
      } catch {
        if (!cancelled) setConnection({ status: "offline" });
      }
    };

    tick();
    const timer = setInterval(tick, POLL_MS);
    return () => {
      cancelled = true;
      clearInterval(timer);
    };
  }, [questions, pushToast]);

  const left = teams.find((t) => t.side === "left") ?? null;
  const right = teams.find((t) => t.side === "right") ?? null;
  const leader =
    left && right && left.score !== right.score ? (left.score > right.score ? "left" : "right") : null;

  const members = useMemo(
    () => ({ left: left?.members ?? [], right: right?.members ?? [] }),
    [left?.members, right?.members],
  );

  const swept = (team) =>
    questions.length > 0 && Object.keys(team?.solves ?? {}).length === questions.length;
  const winner = swept(left) ? "left" : swept(right) ? "right" : null;

  useEffect(() => {
    if (!left || !right) return;
    arenaRef.current?.setColors({
      left: Number(left.color.replace("#", "0x")),
      right: Number(right.color.replace("#", "0x")),
    });
  }, [left?.color, right?.color]);

  useEffect(() => {
    let cancelled = false;

    const tick = async () => {
      try {
        const feed = await api.live(cursorRef.current);
        if (cancelled) return;
        cursorRef.current = feed.cursor ?? cursorRef.current;

        if (feed.chat?.length) {
          setChat((current) => [...current, ...feed.chat].slice(-60));
        }
        setSpectators(feed.spectators ?? []);

        try {
          arenaRef.current?.setClock(feed.now);
          arenaRef.current?.setViewer(user?.id ?? spectator?.id ?? null);
          arenaRef.current?.setSpectators(feed.spectators ?? []);
          feed.chat?.forEach((entry) => {
            arenaRef.current?.spectatorSays(entry.authorId, entry.body);
            if (entry.authorKind === "member") {
              arenaRef.current?.fighterSays(entry.authorId, entry.body);
            }
          });
          feed.emotes?.forEach((entry) =>
            arenaRef.current?.spectatorEmotes(entry.authorId, entry.body),
          );
          feed.misses?.forEach((entry) => {
            const side = sidesRef.current[entry.body];
            if (side) arenaRef.current?.stumble(side);
          });
          if (feed.ghosts?.length) arenaRef.current?.previewGhost();
        } catch {
          /* the scene is decoration; the feed must keep flowing */
        }
      } catch {
        /* the crowd feed is best effort */
      }
    };

    tick();
    const timer = setInterval(tick, LIVE_MS);
    return () => {
      cancelled = true;
      clearInterval(timer);
    };
  }, []);

  useEffect(() => {
    if (!spectator) return;

    const beat = () => {
      if (document.visibilityState !== "visible") return;
      api.heartbeat().catch(() => {});
    };

    beat();
    const timer = setInterval(beat, HEARTBEAT_MS);
    document.addEventListener("visibilitychange", beat);
    return () => {
      clearInterval(timer);
      document.removeEventListener("visibilitychange", beat);
    };
  }, [spectator?.id]);

  const totalPoints = questions.reduce((sum, c) => sum + c.points, 0);
  const ready = Boolean(left && right);

  return (
    <div className={focusMode ? "stage stage--focus" : "stage"}>
      <div className="frame" ref={frameRef}>
        <Arena
          ref={arenaRef}
          leftFalls={Object.keys(right?.solves ?? {}).length}
          rightFalls={Object.keys(left?.solves ?? {}).length}
          members={members}
          winner={winner}
          effects={effects}
        />

        <div className="scrim" />

        <header className="header">
          <h1>{title}</h1>
          <div className="header-actions">
            <span className="pill">
              {questions.length} questions · {totalPoints} pts
            </span>
            {locked && <span className="pill pill--offline">Locked</span>}
            <button type="button" className="link" onClick={() => setFocusMode((v) => !v)}>
              {focusMode ? "Show panels" : "Focus mode"}
            </button>
            {user ? (
              user.role === "admin" ? (
                <a className="link" href="#/admin">
                  Organizer
                </a>
              ) : (
                <a className="link" href="#/me">
                  {user.name}
                </a>
              )
            ) : (
              <a className="link" href="#/login">
                Sign in
              </a>
            )}
          </div>
        </header>

        {connection.status === "online" && !ready && (
          <p className="banner banner--center">
            No contest is set up yet. Sign in as an organizer and create two teams in{" "}
            <a className="link" href="#/admin">
              Organizer
            </a>
            .
          </p>
        )}

        <Scoreboard left={left} right={right} questions={questions} leader={leader} />

        <TeamHud
          team={left}
          questions={questions}
          leading={leader === "left"}
          falls={Object.keys(right?.solves ?? {}).length}
          outcome={winner}
        />
        <TeamHud
          team={right}
          questions={questions}
          leading={leader === "right"}
          falls={Object.keys(left?.solves ?? {}).length}
          outcome={winner}
        />

        {!focusMode && user && (
          <SubmitPanel user={user} teams={teams} questions={questions} locked={locked} />
        )}

        {user?.role === "admin" && !focusMode && <AnimationTester arenaRef={arenaRef} />}

        <SpectatorList people={spectators} viewerId={user?.id ?? spectator?.id ?? null} />

        {!focusMode && (
          <SettingsPanel
            effects={effects}
            onChange={setEffects}
            canSave={Boolean(user || spectator)}
          />
        )}

        {effects.crt && <div className="scanlines" />}

        {user?.role !== "admin" && (
          <CrowdPanel
            chat={chat}
            canEmote={Boolean(spectator)}
            onLeaveSeat={spectator ? onLeaveSeat : null}
          />
        )}

        <aside className="toasts">
          {toasts.map((toast) => (
            <div key={toast.key} className="toast" style={{ borderColor: toast.color }}>
              <strong>{toast.team}</strong>
              <span>{toast.question}</span>
              <em style={{ color: toast.color }}>+{toast.points}</em>
            </div>
          ))}
        </aside>
      </div>
    </div>
  );
}
