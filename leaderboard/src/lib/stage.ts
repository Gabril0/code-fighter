import { useLayoutEffect, useState } from "react";

export const STAGE_WIDTH = 1920;
export const STAGE_HEIGHT = 1080;
export const STAGE_ASPECT = STAGE_WIDTH / STAGE_HEIGHT;

export function useStageScale() {
  const [frame, setFrame] = useState(null);

  useLayoutEffect(() => {
    if (!frame) return undefined;
    const host = frame.parentElement ?? document.documentElement;

    const apply = () => {
      const width = host.clientWidth;
      const height = host.clientHeight;
      if (!width || !height) return;
      const scale = Math.min(width / STAGE_WIDTH, height / STAGE_HEIGHT);
      frame.style.setProperty("--stage-scale", String(scale));
    };

    apply();

    const observer = new ResizeObserver(apply);
    observer.observe(host);
    window.addEventListener("resize", apply);
    window.visualViewport?.addEventListener("resize", apply);

    return () => {
      observer.disconnect();
      window.removeEventListener("resize", apply);
      window.visualViewport?.removeEventListener("resize", apply);
    };
  }, [frame]);

  return setFrame;
}
