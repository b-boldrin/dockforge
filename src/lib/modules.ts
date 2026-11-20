import type { Settings } from "./types";

/**
 * Every block that can sit at the end of the dock follows one pattern: a module has an id,
 * the setting that turns it on, a name, a line about it and an icon. The dock draws them in
 * this order, and where they go depends on the dock, not on the module:
 * - inside the dock ("inline"): bars, islands, and floating docks whose theme keeps them in;
 * - in blocks of their own ("detached"): floating docks whose theme splits them out
 *   (--df-modules: detached), or when the user picks "Separados".
 */
export interface ModuleInfo {
  id: "quick" | "clock";
  key: keyof Settings;
  name: string;
  about: string;
  /** SVG path data, drawn on a 24×24 grid with a stroke. */
  icon: string;
}

export const MODULES: ModuleInfo[] = [
  { id: "quick", key: "widgetQuick", name: "Painel rápido", about: "Volume e as opções do Windows.", icon: "M4 6h10M18 6h2M4 12h4M12 12h8M4 18h12M20 18h0M16 4v4M10 10v4M18 16v4" },
  { id: "clock", key: "widgetClock", name: "Relógio", about: "Hora e data.", icon: "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM12 7v5l3 2" },
];
