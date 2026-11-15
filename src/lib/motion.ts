import type { Settings } from "./types";

const reduced = window.matchMedia("(prefers-reduced-motion: reduce)");
let current: Settings["motion"] = "on";

/** True when animations are off: by choice, or because Windows says so and the user chose to follow it. */
export function motionOff(): boolean {
  return current === "off" || (current === "system" && reduced.matches);
}

function apply() {
  document.documentElement.dataset.motion = motionOff() ? "off" : "on";
}
reduced.addEventListener("change", apply);

/** Tells the page which animation setting is in force. The style sheets read `data-motion`. */
export function setMotion(setting: Settings["motion"]) {
  current = setting;
  apply();
}
