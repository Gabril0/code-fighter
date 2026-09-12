import { useCallback, useEffect, useState } from "react";

import Admin from "./views/Admin";
import Board from "./views/Board";
import Join from "./views/Join";
import Login from "./views/Login";
import Profile from "./views/Profile";
import { api, getSpectator, getToken, setSpectator, setToken } from "./lib/api";

function currentRoute() {
  return (location.hash.replace(/^#/, "") || "/").split("?")[0];
}

function navigate(route) {
  location.hash = route;
}

export default function App() {
  const [route, setRoute] = useState(currentRoute);
  const [user, setUser] = useState(null);
  const [spectator, setSpectatorState] = useState(getSpectator);
  const [resolving, setResolving] = useState(Boolean(getToken()));

  useEffect(() => {
    const onChange = () => setRoute(currentRoute());
    window.addEventListener("hashchange", onChange);
    return () => window.removeEventListener("hashchange", onChange);
  }, []);

  useEffect(() => {
    if (!getToken()) return;
    setResolving(true);
    api
      .me()
      .then((who) => {
        if (who.kind === "spectator") {
          setSpectator(who);
          setSpectatorState(who);
          setUser(null);
          return;
        }
        setSpectator(null);
        setSpectatorState(null);
        setUser(who);
      })
      .catch(() => {
        setToken(null);
        setSpectator(null);
        setSpectatorState(null);
      })
      .finally(() => setResolving(false));
  }, []);

  const handleSignedIn = useCallback((signedIn) => {
    setUser(signedIn);
    if (signedIn.role === "admin") {
      navigate("/admin");
      return;
    }
    navigate(signedIn.profile_set ? "/" : "/me");
  }, []);

  const handleSignedOut = useCallback(() => {
    setUser(null);
    setSpectatorState(null);
    setSpectator(null);
    navigate("/login");
  }, []);

  const handleSpectating = useCallback((guest) => {
    setSpectatorState(guest);
    setUser(null);
    navigate("/");
  }, []);

  const leaveSeat = useCallback(() => {
    setSpectator(null);
    setToken(null);
    setSpectatorState(null);
  }, []);

  if (resolving) return <div className="sheet"><p className="hint">Signing in…</p></div>;

  if (route === "/login") return <Join onSignedIn={handleSignedIn} onSpectating={handleSpectating} />;

  if (route === "/me") {
    if (!user) return <Login onSignedIn={handleSignedIn} />;
    if (user.role === "admin") {
      navigate("/admin");
      return null;
    }
    return <Profile user={user} onUpdated={setUser} onSignedOut={handleSignedOut} />;
  }

  if (route === "/admin") {
    if (!user) return <Login onSignedIn={handleSignedIn} />;
    if (user.role !== "admin") {
      return (
        <div className="sheet">
          <div className="card">
            <h1>Organizer</h1>
            <p className="status status--error">This login is not for an organizer.</p>
            <a className="link" href="#/">
              Back to the scoreboard
            </a>
          </div>
        </div>
      );
    }
    return <Admin onSignedOut={handleSignedOut} />;
  }

  if (!user && !spectator) {
    return <Join onSignedIn={handleSignedIn} onSpectating={handleSpectating} />;
  }

  return <Board user={user} spectator={spectator} onLeaveSeat={leaveSeat} />;
}
