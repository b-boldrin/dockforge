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
  id: "media" | "system" | "pomodoro" | "world" | "quick" | "clock";
  key: keyof Settings;
  name: string;
  about: string;
  /** SVG path data, drawn on a 24×24 grid with a stroke. */
  icon: string;
  /** Integrations count toward a saved bar's limit; the quick panel does not. */
  integration: boolean;
}

export const MODULES: ModuleInfo[] = [
  { id: "media", key: "widgetMedia", name: "Tocando agora", about: "Música ou vídeo em reprodução, com os controles.", icon: "M9 18V6l10-2v12M9 18a3 3 0 1 1-6 0 3 3 0 0 1 6 0zM19 16a3 3 0 1 1-6 0 3 3 0 0 1 6 0z", integration: true },
  { id: "system", key: "widgetSystem", name: "Sistema", about: "Processador, memória e bateria.", icon: "M4 19h16M7 16V10M12 16V5M17 16v-4", integration: true },
  { id: "pomodoro", key: "widgetPomodoro", name: "Pomodoro", about: "25 minutos de foco e 5 de pausa.", icon: "M12 21a8 8 0 1 0 0-16 8 8 0 0 0 0 16zM12 9v4l2.5 2M9 2h6", integration: true },
  { id: "world", key: "widgetWorldClock", name: "Hora em outra cidade", about: "A hora de uma cidade escolhida.", icon: "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM3 12h18M12 3c2.5 2.6 3.8 5.6 3.8 9s-1.3 6.4-3.8 9c-2.5-2.6-3.8-5.6-3.8-9S9.5 5.6 12 3z", integration: true },
  { id: "quick", key: "widgetQuick", name: "Painel rápido", about: "Volume, bateria e as opções do Windows.", icon: "M4 6h10M18 6h2M4 12h4M12 12h8M4 18h12M20 18h0M16 4v4M10 10v4M18 16v4", integration: false },
  { id: "clock", key: "widgetClock", name: "Relógio", about: "Hora e data.", icon: "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM12 7v5l3 2", integration: true },
];

export const INTEGRATIONS = MODULES.filter((m) => m.integration);
