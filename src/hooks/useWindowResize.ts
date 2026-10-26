import { useEffect } from "react";
import { api } from "@/lib/api";

const BAR_HEIGHT = 58;
const EXPANDED_HEIGHT = 600;

const isAnyPopoverOpen = () => document.querySelectorAll("[data-radix-popper-content-wrapper]").length > 0;

/**
 * Same behaviour as Pluely: the bar window stays 54px tall and grows only
 * while a popover (reading panel, voice settings) is open beneath it.
 */
export const useWindowResize = () => {
  useEffect(() => {
    let expanded = false;
    const sync = () => {
      const open = isAnyPopoverOpen();
      if (open !== expanded) {
        expanded = open;
        api.setWindowHeight(open ? EXPANDED_HEIGHT : BAR_HEIGHT).catch(() => {});
      }
    };
    const observer = new MutationObserver(sync);
    observer.observe(document.body, { childList: true, subtree: true, attributes: true, attributeFilter: ["data-state"] });
    return () => observer.disconnect();
  }, []);
};
