const BASE_URL = import.meta.env.VITE_API_URL ?? "http://localhost:8080";
const TOKEN_KEY = "code-fighter-token";
const SPECTATOR_KEY = "code-fighter-spectator";

export function getToken() {
  return localStorage.getItem(TOKEN_KEY);
}

export function setToken(token) {
  if (token) localStorage.setItem(TOKEN_KEY, token);
  else localStorage.removeItem(TOKEN_KEY);
}

export function getSpectator() {
  try {
    const raw = localStorage.getItem(SPECTATOR_KEY);
    return raw ? JSON.parse(raw) : null;
  } catch {
    return null;
  }
}

export function setSpectator(spectator) {
  if (spectator) localStorage.setItem(SPECTATOR_KEY, JSON.stringify(spectator));
  else localStorage.removeItem(SPECTATOR_KEY);
}

async function request(method: string, path: string, body?: any, { auth = false }: { auth?: boolean } = {}) {
  const headers: Record<string, string> = {};
  if (body !== undefined) headers["Content-Type"] = "application/json";
  if (auth) {
    const token = getToken();
    if (!token) throw new Error("not signed in");
    headers.Authorization = `Bearer ${token}`;
  }

  const response = await fetch(`${BASE_URL}${path}`, {
    method,
    headers,
    body: body === undefined ? undefined : JSON.stringify(body),
  });

  const text = await response.text();
  const payload = text ? JSON.parse(text) : null;
  if (!response.ok) {
    throw new Error(payload?.error ?? `request failed (${response.status})`);
  }
  return payload;
}

async function download(path, fallbackName) {
  const token = getToken();
  if (!token) throw new Error("sign in before downloading");

  const response = await fetch(`${BASE_URL}${path}`, {
    headers: { Authorization: `Bearer ${token}` },
  });
  if (!response.ok) {
    const text = await response.text();
    let message = `download failed (${response.status})`;
    try {
      message = JSON.parse(text)?.error ?? message;
    } catch {
      /* the body was not JSON */
    }
    throw new Error(message);
  }

  const disposition = response.headers.get("content-disposition") ?? "";
  const name = disposition.match(/filename="([^"]+)"/)?.[1] ?? fallbackName;
  const blob = await response.blob();
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = name;
  document.body.appendChild(anchor);
  anchor.click();
  anchor.remove();
  URL.revokeObjectURL(url);
  return name;
}

export const api = {
  health: () => request("GET", "/health"),
  questions: () => request("GET", "/v1/questions"),
  question: (id) => request("GET", `/v1/questions/${id}`, undefined, { auth: true }),
  saveEffects: (effects) => request("PUT", "/v1/me/effects", effects, { auth: true }),
  heartbeat: () => request("POST", "/v1/heartbeat", undefined, { auth: true }),
  downloadPack: (id, teamId) =>
    download(
      `/v1/questions/${id}/pack${teamId ? `?team=${encodeURIComponent(teamId)}` : ""}`,
      `${id}.zip`,
    ),
  board: () => request("GET", "/v1/board"),
  authenticate: (token) => request("POST", "/v1/auth", { token }),
  spectate: ({ name, photo }) => request("POST", "/v1/spectate", { name, photo }),
  live: (since = 0) => request("GET", `/v1/live?since=${since}`),
  say: (body) => request("POST", "/v1/chat", { body }, { auth: true }),
  emote: (body) => request("POST", "/v1/emotes", { body }, { auth: true }),
  summonGhost: () => request("POST", "/v1/ghost", undefined, { auth: true }),
  me: () => request("GET", "/v1/me", undefined, { auth: true }),
  updateMe: (patch) => request("PATCH", "/v1/me", patch, { auth: true }),
  submit: (questionId: any, { outputs, apiUrl, teamId }: any = {}) =>
    request(
      "POST",
      "/v1/submissions",
      {
        question_id: questionId,
        outputs: outputs ?? {},
        api_url: apiUrl ?? null,
        team_id: teamId ?? null,
      },
      { auth: true },
    ),
  admin: {
    state: () => request("GET", "/v1/admin/state", undefined, { auth: true }),
    createUser: (name: any, teamId?: any) =>
      request("POST", "/v1/admin/users", { name, team_id: teamId ?? null }, { auth: true }),
    patchUser: (id, patch) => request("PATCH", `/v1/admin/users/${id}`, patch, { auth: true }),
    deleteUser: (id) => request("DELETE", `/v1/admin/users/${id}`, undefined, { auth: true }),
    createTeam: (name: any, color?: any) => request("POST", "/v1/admin/teams", { name, color }, { auth: true }),
    patchTeam: (id, patch) => request("PATCH", `/v1/admin/teams/${id}`, patch, { auth: true }),
    deleteTeam: (id) => request("DELETE", `/v1/admin/teams/${id}`, undefined, { auth: true }),
    setLock: (locked) => request("POST", "/v1/admin/lock", { locked }, { auth: true }),
    setTitle: (title) => request("POST", "/v1/admin/title", { title }, { auth: true }),
    reset: () => request("POST", "/v1/admin/reset", undefined, { auth: true }),
  },
};

export const API_BASE_URL = BASE_URL;
