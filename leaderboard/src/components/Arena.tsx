import { forwardRef, useEffect, useImperativeHandle, useRef, useState } from "react";

import { Ring } from "../three/ring";

const Arena = forwardRef(function Arena({ leftFalls, rightFalls, members, winner, effects }: any, ref) {
  const containerRef = useRef(null);
  const ringRef = useRef(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const ring = new Ring(containerRef.current);
    ringRef.current = ring;
    const settle = () => {
      if (ringRef.current === ring) setLoading(false);
    };
    ring.ready.then(settle).catch(settle);
    return () => {
      ring.dispose();
      ringRef.current = null;
    };
  }, []);

  const rosterKey = JSON.stringify(members ?? {});

  useEffect(() => {
    if (loading || !members?.left?.length) return;
    ringRef.current?.setRoster(members);
    ringRef.current?.seat("left", leftFalls);
    ringRef.current?.seat("right", rightFalls);
    ringRef.current?.restageOutcome();
  }, [loading, rosterKey]);

  useEffect(() => {
    if (loading) return;
    ringRef.current?.setFalls("left", leftFalls);
  }, [loading, leftFalls]);

  useEffect(() => {
    if (loading) return;
    ringRef.current?.setFalls("right", rightFalls);
  }, [loading, rightFalls]);

  useEffect(() => {
    if (loading) return;
    ringRef.current?.setOutcome(winner ?? null);
  }, [loading, winner]);

  useEffect(() => {
    if (loading) return;
    ringRef.current?.setEffects(effects);
  }, [loading, effects]);

  useImperativeHandle(ref, () => ({
    punch: (side) => ringRef.current?.punch(side),
    stumble: (side) => ringRef.current?.stumble(side),
    setColors: (colors) => ringRef.current?.setColors(colors),
    previewClip: (side, clip) => ringRef.current?.previewClip(side, clip),
    previewGhost: () => ringRef.current?.previewGhost(),
    setSpectators: (people) => ringRef.current?.setSpectators(people),
    setClock: (now) => ringRef.current?.setClock(now),
    setViewer: (id) => ringRef.current?.setViewer(id),
    fighterSays: (id, message) => ringRef.current?.fighterSays(id, message),
    spectatorSays: (id, message) => ringRef.current?.spectatorSays(id, message),
    spectatorEmotes: (id, emote) => ringRef.current?.spectatorEmotes(id, emote),
    burst: (kind, side) => ringRef.current?.previewBurst(kind, side),
  }));

  return (
    <>
      <div className="arena" ref={containerRef} />
      {loading && <div className="arena-loading">Loading fighters…</div>}
    </>
  );
});

export default Arena;
