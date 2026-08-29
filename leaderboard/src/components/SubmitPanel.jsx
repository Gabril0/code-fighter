import { useEffect, useRef, useState } from "react";

import Statement from "./Statement";
import { api } from "../lib/api";

const CLOSE_AFTER_MS = 2600;
const SHOWN_FAILURES = 12;

function caseKey(filename) {
  return filename
    .split("/")
    .pop()
    .replace(/\.(out|txt)$/i, "")
    .trim();
}

function QuestionIcon() {
  return (
    <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true">
      <path
        d="M6 3h9l4 4v14H6z"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinejoin="round"
      />
      <path d="M9 12h6M9 16h4" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
    </svg>
  );
}

export default function SubmitPanel({ user, teams, questions, locked }) {
  const isAdmin = user.role === "admin";
  const [open, setOpen] = useState(false);
  const [teamId, setTeamId] = useState(user.team_id ?? "");
  const [questionId, setQuestionId] = useState("");
  const [detail, setDetail] = useState(null);
  const [showStatement, setShowStatement] = useState(false);
  const [outputs, setOutputs] = useState({});
  const [apiUrl, setApiUrl] = useState("http://127.0.0.1:8000");
  const [result, setResult] = useState(null);
  const [error, setError] = useState(null);
  const [busy, setBusy] = useState(false);
  const fileRef = useRef(null);
  const closeRef = useRef(null);

  useEffect(() => {
    if (!questionId && questions.length > 0) setQuestionId(questions[0].id);
  }, [questions, questionId]);

  useEffect(() => {
    if (isAdmin && !teamId && teams.length > 0) setTeamId(teams[0].id);
  }, [isAdmin, teams, teamId]);

  useEffect(() => {
    if (!questionId) return;
    let cancelled = false;
    setDetail(null);
    api
      .question(questionId)
      .then((payload) => {
        if (!cancelled) setDetail(payload);
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [questionId]);

  useEffect(() => () => clearTimeout(closeRef.current), []);

  const targetTeam = teams.find((team) => team.id === (isAdmin ? teamId : user.team_id));
  const solvedPrefix = questions.findIndex((item) => !targetTeam?.solves?.[item.id]);
  const openThrough = isAdmin
    ? questions.length - 1
    : solvedPrefix === -1
      ? questions.length - 1
      : solvedPrefix;
  const isOpen = (index) => index <= openThrough;
  const question = questions.find((item) => item.id === questionId) ?? null;
  useEffect(() => {
    const at = questions.findIndex((item) => item.id === questionId);
    if (at > -1 && !isOpen(at)) setQuestionId(questions[openThrough]?.id ?? questions[0]?.id);
  }, [questionId, openThrough, questions]);

  const solved = Boolean(targetTeam?.solves?.[questionId]);
  const tries = targetTeam?.attempts?.[questionId] ?? 0;
  const solvedCount = Object.keys(targetTeam?.solves ?? {}).length;
  const isApi = question?.mode === "api";

  function pickQuestion(id) {
    setQuestionId(id);
    setOutputs({});
    setResult(null);
    setError(null);
    setShowStatement(false);
    if (fileRef.current) fileRef.current.value = "";
  }

  async function attach(event) {
    const files = [...(event.target.files ?? [])];
    const collected = {};
    for (const file of files) {
      collected[caseKey(file.name)] = await file.text();
    }
    setOutputs(collected);
    setResult(null);
    setError(null);
  }

  function grabStatement() {
    if (!detail?.statement) return;
    const blob = new Blob([detail.statement], { type: "text/markdown;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = `${questionId}-statement.md`;
    document.body.appendChild(anchor);
    anchor.click();
    anchor.remove();
    URL.revokeObjectURL(url);
  }

  async function grabPack() {
    setError(null);
    try {
      await api.downloadPack(questionId, isAdmin ? teamId : undefined);
    } catch (problem) {
      setError(problem.message);
    }
  }

  async function send(event) {
    event.preventDefault();
    setBusy(true);
    setError(null);
    try {
      const payload = await api.submit(questionId, {
        outputs: isApi ? undefined : outputs,
        apiUrl: isApi ? apiUrl : undefined,
        teamId: isAdmin ? teamId : undefined,
      });
      setResult(payload);
      if (payload.correct) {
        closeRef.current = setTimeout(() => {
          setOpen(false);
          setResult(null);
          setOutputs({});
          if (fileRef.current) fileRef.current.value = "";
        }, CLOSE_AFTER_MS);
      }
    } catch (problem) {
      setError(problem.message);
    } finally {
      setBusy(false);
    }
  }

  if (!isAdmin && !user.team_id) return null;

  const failures = (result?.cases ?? []).filter((item) => !item.ok);
  const attached = Object.keys(outputs).length;
  const missing = (detail?.caseList ?? [])
    .filter((item) => !(item.name in outputs))
    .map((item) => item.name);

  return (
    <div className="desk">
      {open && (
        <section className="desk-panel">
          <div className="desk-tabs">
            {questions.map((item, index) => {
              const done = Boolean(targetTeam?.solves?.[item.id]);
              const attempts = targetTeam?.attempts?.[item.id] ?? 0;
              const open = isOpen(index);
              return (
                <button
                  key={item.id}
                  type="button"
                  disabled={!open}
                  className={[
                    "desk-tab",
                    item.id === questionId ? "desk-tab--on" : "",
                    done ? "desk-tab--done" : "",
                    open ? "" : "desk-tab--locked",
                  ]
                    .filter(Boolean)
                    .join(" ")}
                  onClick={() => open && pickQuestion(item.id)}
                  title={open ? item.title : "Solve the previous one to unlock"}
                >
                  {open ? item.id.toUpperCase() : "🔒"}
                  {open && attempts > 0 && <em>{attempts}</em>}
                </button>
              );
            })}
          </div>

          {question && (
            <header className="desk-head">
              <div>
                <strong>{question.title}</strong>
                <span className="hint">
                  {question.difficulty} · {question.points} pts ·{" "}
                  {isApi ? "checked through the API" : `${question.cases} cases`}
                </span>
              </div>
              <div className="desk-head-actions">
                <button type="button" className="link" onClick={() => setShowStatement((v) => !v)}>
                  {showStatement ? "Close" : "Statement"}
                </button>
                <button
                  type="button"
                  className="link"
                  onClick={grabStatement}
                  disabled={!detail?.statement}
                  title="Download statement.md"
                >
                  Download .md
                </button>
              </div>
            </header>
          )}

          {showStatement && detail && (
            <div className="desk-statement">
              <Statement markdown={detail.statement} />
            </div>
          )}

          {isAdmin && (
            <label className="desk-field">
              Team
              <select value={teamId} onChange={(event) => setTeamId(event.target.value)}>
                {teams.map((team) => (
                  <option key={team.id} value={team.id}>
                    {team.name}
                  </option>
                ))}
              </select>
            </label>
          )}

          <form className="desk-form" onSubmit={send}>
            {isApi ? (
              <>
                <div className="desk-actions">
                  <button type="button" onClick={grabPack}>
                    Download skeleton
                  </button>
                </div>
                <label className="desk-field">
                  Your API address
                  <input
                    value={apiUrl}
                    onChange={(event) => setApiUrl(event.target.value)}
                    placeholder="http://192.168.0.10:8000"
                    spellCheck={false}
                  />
                </label>
                <p className="hint">
                  The package includes the skeleton in <code>skeleton/</code> and{" "}
                  <code>test_api.py</code> to check before submitting. The simplest way to
                  expose your API is a tunnel:{" "}
                  <code>cloudflared tunnel --url http://localhost:8000</code> or{" "}
                  <code>ngrok http 8000</code>, then paste here the <code>https://</code> address it
                  prints. Details are in the package's HOW_TO_SUBMIT.md.
                </p>
              </>
            ) : (
              <>
                <div className="desk-actions">
                  <button type="button" onClick={grabPack}>
                    Download inputs
                  </button>
                  <button type="button" onClick={() => fileRef.current?.click()}>
                    Attach outputs
                  </button>
                </div>
                <input
                  ref={fileRef}
                  className="desk-hidden"
                  type="file"
                  multiple
                  onChange={attach}
                />
                <p className="hint">
                  {attached === 0
                    ? `No files attached · needs ${question?.cases ?? 0}`
                    : missing.length
                      ? `${attached} of ${question?.cases ?? 0} · missing ${
                          missing.length > 4
                            ? `${missing.slice(0, 4).join(", ")} and ${missing.length - 4} more`
                            : missing.join(", ")
                        }`
                      : `${attached} file(s) · complete`}
                </p>
              </>
            )}

            {solved && <p className="hint">This question is already solved.</p>}
            {locked && <p className="hint">Submissions are locked by the organizer.</p>}
            {tries > 0 && !solved && (
              <p className="hint">
                {tries} attempt{tries > 1 ? "s" : ""}; wrong answers do not lose points.
              </p>
            )}

            <button type="submit" disabled={busy || locked || solved}>
              {busy ? "Checking…" : isApi ? "Check API" : "Submit outputs"}
            </button>
          </form>

          {error && <p className="status status--error">{error}</p>}

          {result && (
            <div className={result.correct ? "desk-result desk-result--ok" : "desk-result"}>
              <strong>
                {result.correct
                  ? "Passed! The opponent went down."
                  : `${result.passed ?? 0} of ${result.total ?? 0} cases`}
              </strong>
              {!result.correct && failures.length > 0 && (
                <>
                  <div className="desk-cases">
                    {failures.slice(0, SHOWN_FAILURES).map((item) => (
                      <span key={item.name} className="desk-case" title={item.detail ?? ""}>
                        {item.name}
                      </span>
                    ))}
                    {failures.length > SHOWN_FAILURES && (
                      <span className="desk-case desk-case--more">
                        +{failures.length - SHOWN_FAILURES}
                      </span>
                    )}
                  </div>
                  <p className="hint">
                    {failures.length === 1 ? "1 case failed" : `${failures.length} cases failed`}
                  </p>
                </>
              )}
              {!result.correct &&
                failures
                  .filter((item) => item.detail)
                  .slice(0, 3)
                  .map((item) => (
                    <p key={item.name} className="hint">
                      <strong>{item.name}</strong>: {item.detail}
                    </p>
                  ))}
            </div>
          )}
        </section>
      )}

      <button
        type="button"
        className={open ? "desk-button desk-button--on" : "desk-button"}
        onClick={() => setOpen((value) => !value)}
      >
        <QuestionIcon />
        <span>Questions</span>
        <em>
          {solvedCount}/{questions.length}
        </em>
      </button>
    </div>
  );
}
