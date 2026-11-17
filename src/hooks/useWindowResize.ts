import { RefObject, useEffect } from "react";
import { api } from "@/lib/api";

/** Room the window takes beneath (or above) the panel while a popover is open */
const POPOVER_ROOM = 542;

const isAnyPopoverOpen = () => document.querySelectorAll("[data-radix-popper-content-wrapper]").length > 0;

/**
 * Same behaviour as Pluely: the bar window hugs the panel (one row, or two
 * while drawing) and grows only while a popover (reading panel, voice
 * settings, drawing colours) is open. `up` grows it upwards instead, for a
 * bar near the bottom of the screen. `layout` changes whenever the panel
 * element is swapped (full bar vs. pill), so the new one is measured.
 */
export const useWindowResize = (panel: RefObject<HTMLElement | null>, up: boolean, layout: string) => {
  useEffect(() => {
    let last = "";
    const sync = () => {
      const el = panel.current;
      if (!el) return;
      const height = Math.ceil(el.offsetHeight) + (isAnyPopoverOpen() ? POPOVER_ROOM : 0);
      const key = `${height}|${up}`;
      if (height && key !== last) {
        last = key;
        api.setWindowHeight(height, up).catch(() => {});
      }
    };
    sync();
    const mo = new MutationObserver(sync);
    mo.observe(document.body, { childList: true, subtree: true, attributes: true, attributeFilter: ["data-state"] });
    const ro = new ResizeObserver(sync);
    if (panel.current) ro.observe(panel.current);
    return () => {
      mo.disconnect();
      ro.disconnect();
    };
  }, [panel, up, layout]);
};
