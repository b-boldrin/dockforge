import type { Settings } from "../lib/types";

/** A ready-made dock: a theme plus where it sits, and how it behaves.
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
  fill: null,
  radius: null,
  opacity: null,
  gap: null,
  appScale: 1,
  indicator: "theme",
  align: "center",
  autoHide: true,
  separator: true,
  labels: false,
  groupsEnabled: false,
};

/** The first ones are the styles people most often give their Windows taskbar. */
export const PRESETS: Preset[] = [
  {
    id: "barra-classica",
    name: "Barra clássica",
    about: "Reta, opaca, de ponta a ponta, com o nome de cada app aberto ao lado do ícone.",
    settings: { ...clean, theme: "barra-classica", layout: "bar", autoHide: false, reserveSpace: true, position: "bottom", align: "start", iconSize: 36, magnify: 1, labels: true, separator: false },
  },
  {
    id: "centralizada",
    name: "Centralizada",
    about: "Barra transparente com os ícones no meio, para o papel de parede aparecer.",
    settings: { ...clean, theme: "transparente", layout: "bar", autoHide: false, reserveSpace: true, position: "bottom", iconSize: 40, magnify: 1.1 },
  },
  {
    id: "barra-flutuante",
    name: "Barra flutuante",
    about: "De ponta a ponta, mas solta das bordas da tela e com os cantos arredondados.",
    settings: { ...clean, theme: "barra-flutuante", layout: "bar", autoHide: false, reserveSpace: true, position: "bottom", iconSize: 40, magnify: 1.1 },
  },
  {
    id: "retro",
    name: "Retrô",
    about: "Cinza, com relevo e botões de apertar, como nos computadores dos anos 90.",
    settings: { ...clean, theme: "retro-cinza", layout: "bar", autoHide: false, reserveSpace: true, position: "bottom", align: "start", iconSize: 32, magnify: 1, labels: true, separator: false },
  },
  {
    id: "prateleira",
    name: "Prateleira",
    about: "Vidro fosco no centro, com reflexo e ícones que crescem bastante quando o mouse passa.",
    settings: { ...clean, theme: "prateleira", layout: "float", position: "bottom", iconSize: 52, magnify: 1.5 },
  },
  {
    id: "ilha",
    name: "Ilha",
    about: "Uma pílula no topo com a música e a hora. Os apps aparecem quando você aponta para ela.",
    settings: { ...clean, theme: "midnight-pill", layout: "island", position: "top", iconSize: 36, magnify: 1.15 },
  },
  {
    id: "coluna",
    name: "Coluna de foco",
    about: "Fina, na lateral esquerda, e some sozinha para a tela ficar livre.",
    settings: { ...clean, theme: "forge-glass", layout: "float", position: "left", iconSize: 40, magnify: 1.15, autoHide: true, separator: false },
  },
  {
    id: "terminal",
    name: "Terminal",
    about: "Barra âmbar no topo, com cara de monitor antigo e os números da máquina.",
    settings: { ...clean, theme: "ambar-crt", layout: "bar", autoHide: false, reserveSpace: true, position: "top", align: "start", iconSize: 36, magnify: 1 },
  },
  {
    id: "mesas",
    name: "Quatro mesas",
    about: "Grupos de apps na mesma dock, com clima de Natal. Alt+1 a Alt+4 troca de mesa.",
    settings: { ...clean, theme: "natal-cristal", layout: "float", position: "bottom", align: "start", iconSize: 44, magnify: 1.25, groupsEnabled: true },
  },
];

/** True when the dock on screen is exactly this preset. */
export function matches(settings: Settings, preset: Preset): boolean {
  return Object.entries(preset.settings).every(([key, value]) => settings[key as keyof Settings] === value);
}
