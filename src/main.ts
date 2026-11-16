import { mount } from "svelte";
import { getCurrentWindow } from "@tauri-apps/api/window";

// One bundle, two windows: the dock itself, and the settings window ("hub").
// The window label says which one this is (the ?view=hub address is for browser previews).
function currentView(): string | null {
  try {
    return getCurrentWindow().label;
  } catch {
    return new URLSearchParams(location.search).get("view");
  }
}
const view = currentView();
const target = document.getElementById("app")!;

// If something breaks before the screen is drawn, say so on screen instead of leaving it empty.
function showFailure(err: unknown) {
  const box = document.createElement("pre");
  box.setAttribute("role", "alert");
  box.style.cssText = "margin:0;padding:24px;color:#fff;background:#7a1f1f;font:13px/1.5 Consolas,monospace;white-space:pre-wrap;position:fixed;inset:0;overflow:auto;z-index:99999";
  const detail = err instanceof Error ? `${err.message}\n\n${err.stack ?? ""}` : String(err);
  box.textContent = `O DockForge não conseguiu abrir esta janela (${view ?? "?"}).\n\n${detail}`;
  document.body.append(box);
}
// The web view's own right-click menu (Back, Reload, Inspect) has no place in an app.
// Text fields keep theirs, for copy and paste.
window.addEventListener("contextmenu", (e) => {
  if (!(e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement)) e.preventDefault();
});
window.addEventListener("error", (e) => {
  // Browsers raise this harmless notice when a ResizeObserver resizes something in the same pass.
  if (e.message.startsWith("ResizeObserver loop")) {
    e.stopImmediatePropagation();
    return;
  }
  showFailure(e.error ?? e.message);
});
window.addEventListener("unhandledrejection", (e) => showFailure(e.reason));

try {
  if (view === "quick") {
    await import("./quick/quick.css");
    const { setMotion } = await import("./lib/motion");
    const { invoke } = await import("@tauri-apps/api/core");
    invoke<{ motion: string }>("get_settings").then((s) => setMotion(s.motion as never)).catch(() => {});
    const { default: Quick } = await import("./quick/Quick.svelte");
    mount(Quick, { target });
  } else if (view === "hub") {
    await import("./hub/hub.css");
    const { default: Hub } = await import("./hub/Hub.svelte");
    mount(Hub, { target });
  } else {
    await import("./lib/base.css");
    await import("./lib/dock-window.css");
    const { default: App } = await import("./App.svelte");
    mount(App, { target });
  }
} catch (err) {
  showFailure(err);
}
