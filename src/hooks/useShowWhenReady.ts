import { useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

/**
 * Overlay windows are created hidden and show themselves only after their
 * first frame is painted, so the screen never flashes a white box while the
 * page loads.
 */
export const useShowWhenReady = (focus = false) => {
  useEffect(() => {
    // Two frames: the first lays out, the second is on screen
    const id = requestAnimationFrame(() =>
      requestAnimationFrame(() => {
        const w = getCurrentWindow();
        w.show()
          .then(() => (focus ? w.setFocus() : undefined))
          .catch(() => {});
      })
    );
    return () => cancelAnimationFrame(id);
  }, [focus]);
};
