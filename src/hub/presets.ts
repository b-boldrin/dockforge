import type { Settings } from "../lib/types";

/** A ready-made dock: a theme plus where it sits, how it behaves and which widgets it shows.
 *  One click applies everything; every choice can still be changed afterwards. */
export interface Preset {
  id: string;
  name: string;
  about: string;
  settings: Partial<Settings>;
}

/** What every preset resets, so nothing is left over from the previous dock. */
const clean: Partial<Settings> = {
  accent: null,
  radius: null,
  opacity: null,
  align: "center",
  autoHide: false,
  separator: true,
  widgetMedia: false,
  widgetClock: false,
  widgetSystem: false,
};

export const PRESETS: Preset[] = [
  {
    id: "classica",
    name: "Clássica",
    about: "Flutua no centro, embaixo, e os ícones crescem quando o mouse passa.",
    settings: { ...clean, theme: "forge-glass", layout: "float", position: "bottom", iconSize: 48, magnify: 1.35, widgetClock: true },
  },
  {
    id: "ilha",
    name: "Ilha",
    about: "Uma pílula no topo com a música e a hora. Os apps aparecem quando você aponta para ela.",
    settings: { ...clean, theme: "midnight-pill", layout: "island", position: "top", iconSize: 36, magnify: 1.15, widgetMedia: true, widgetClock: true },
  },
  {
    id: "painel",
    name: "Painel",
    about: "Barra de ponta a ponta, com processador, memória e relógio no fim.",
    settings: { ...clean, theme: "papel", layout: "bar", position: "bottom", iconSize: 40, magnify: 1, widgetSystem: true, widgetClock: true },
  },
  {
    id: "coluna",
    name: "Coluna de foco",
    about: "Fina, na lateral esquerda, e some sozinha para a tela ficar livre.",
    settings: { ...clean, theme: "forge-glass", layout: "float", position: "left", iconSize: 40, magnify: 1.15, autoHide: true, separator: false },
  },
  {
    id: "canto",
    name: "Canto do som",
    about: "Pequena, no canto direito, com o que está tocando sempre à mão.",
    settings: { ...clean, theme: "noite-de-natal", layout: "float", position: "bottom", align: "end", iconSize: 36, magnify: 1.2, widgetMedia: true },
  },
  {
    id: "terminal",
    name: "Terminal",
    about: "Barra âmbar no topo, com cara de monitor antigo e os números da máquina.",
    settings: { ...clean, theme: "ambar-crt", layout: "bar", position: "top", iconSize: 36, magnify: 1, widgetSystem: true, widgetClock: true },
  },
  {
    id: "mesas",
    name: "Quatro mesas",
    about: "Quatro grupos de apps na mesma dock. Alt+1 a Alt+4 troca de mesa.",
    settings: { ...clean, theme: "natal-cristal", layout: "float", position: "bottom", align: "start", iconSize: 44, magnify: 1.25, groupsEnabled: true, widgetClock: true },
  },
];

/** True when the dock on screen is exactly this preset. */
export function matches(settings: Settings, preset: Preset): boolean {
  return Object.entries(preset.settings).every(([key, value]) => settings[key as keyof Settings] === value);
}
