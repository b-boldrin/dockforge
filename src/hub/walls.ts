// Little painted landscapes used behind theme previews, so a theme is seen the way it will be
// used: on top of a wallpaper. Pure decoration, drawn as SVG.

type Wall = {
  sky: [string, string];
  hill: string;
  ground: string;
  trees?: boolean;
  snow?: boolean;
  moon?: boolean;
  /** A big red sun low in the sky (instead of the moon). */
  sun?: boolean;
  /** A snow-capped mountain behind the hills. */
  peak?: boolean;
  /** Sparks from a forge, floating up from the ground. */
  embers?: boolean;
};

const WALLS: Record<string, Wall> = {
  "forja-samurai": { sky: ["#f6efe2", "#ead9c2"], hill: "#2b2422", ground: "#1c1917", sun: true, peak: true, embers: true },
  "natal-cristal": { sky: ["#0d2f3a", "#2a6a72"], hill: "#123b37", ground: "#dfeeee", trees: true, snow: true, moon: true },
  "noite-de-natal": { sky: ["#1a0f24", "#4d1d2c"], hill: "#2a1220", ground: "#e9dccf", trees: true, snow: true, moon: true },
  "forge-glass": { sky: ["#1c2230", "#3c4a66"], hill: "#232a3a", ground: "#151a24", moon: true },
  "barra-classica": { sky: ["#0f3f73", "#3b8fd4"], hill: "#0c2f57", ground: "#0a2646" },
  transparente: { sky: ["#3a2a5c", "#e58a6b"], hill: "#2a1d44", ground: "#1c1330", moon: true },
  "barra-flutuante": { sky: ["#16202e", "#34506b"], hill: "#101a26", ground: "#0b131c", moon: true },
  "retro-cinza": { sky: ["#008080", "#008080"], hill: "#008080", ground: "#008080" },
  prateleira: { sky: ["#7fb6e8", "#f3d9c4"], hill: "#5d8fb8", ground: "#3f6f95" },
  papel: { sky: ["#f1d9b8", "#f7ecd9"], hill: "#c9b08a", ground: "#a98f6b" },
  "ambar-crt": { sky: ["#0d0802", "#2a1a04"], hill: "#1c1103", ground: "#0a0601", moon: true },
  "midnight-pill": { sky: ["#120c22", "#3a1d5c"], hill: "#1d1236", ground: "#0f0a1c", moon: true },
};

function hashHue(id: string) {
  let h = 0;
  for (const ch of id) h = (h * 31 + ch.charCodeAt(0)) % 360;
  return h;
}

function wallFor(id: string): Wall {
  const known = WALLS[id];
  if (known) return known;
  const h = hashHue(id);
  return { sky: [`hsl(${h} 35% 18%)`, `hsl(${(h + 30) % 360} 40% 38%)`], hill: `hsl(${h} 30% 14%)`, ground: `hsl(${h} 25% 10%)`, moon: true };
}

const cache = new Map<string, string>();

/** A CSS background-image value with the landscape for this theme. */
export function wallpaper(id: string): string {
  const hit = cache.get(id);
  if (hit) return hit;
  const w = wallFor(id);
  let s =
    "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 640 360' preserveAspectRatio='xMidYMid slice'>" +
    `<defs><linearGradient id='k' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='${w.sky[0]}'/><stop offset='1' stop-color='${w.sky[1]}'/></linearGradient></defs>` +
    "<rect width='640' height='360' fill='url(#k)'/>";
  if (w.moon) {
    s += "<circle cx='500' cy='78' r='26' fill='#f4f1e4' opacity='.92'/>";
    for (let i = 0; i < 26; i++)
      s += `<circle cx='${(i * 97 + 31) % 640}' cy='${(i * 53 + 9) % 150}' r='${i % 3 === 0 ? 1.4 : 0.8}' fill='#fff' opacity='.7'/>`;
  }
  if (w.sun) s += "<circle cx='430' cy='150' r='78' fill='#c41e3a'/>";
  if (w.peak)
    s +=
      `<path d='M150 250 L330 92 L520 250z' fill='${w.hill}' opacity='.82'/>` +
      "<path d='M330 92 L292 126 L310 122 L322 134 L336 120 L352 130 L368 126z' fill='#f6efe2'/>";
  s += `<path d='M0 238 Q140 196 300 222 T640 206 V360 H0z' fill='${w.hill}'/>`;
  if (w.trees) {
    const xs = [34, 70, 118, 160, 214, 410, 452, 500, 548, 598, 620];
    xs.forEach((x, j) => {
      const h = 64 + ((j * 29) % 54);
      const base = 262 + (j % 3) * 6;
      const wd = h * 0.34;
      const tri = `M${x} ${base - h} L${x - wd} ${base} L${x + wd} ${base}z`;
      s += `<path d='${tri}' fill='${w.hill}'/><path d='${tri}' fill='#000' opacity='.18'/>`;
      if (w.snow)
        s += `<path d='M${x} ${base - h} L${x - wd * 0.32} ${base - h * 0.68} L${x + wd * 0.32} ${base - h * 0.68}z' fill='#fff' opacity='.85'/>`;
    });
  }
  s += `<path d='M0 300 Q180 272 360 292 T640 282 V360 H0z' fill='${w.ground}'/>`;
  if (w.embers)
    for (let f = 0; f < 34; f++)
      s += `<circle cx='${(f * 83 + 29) % 640}' cy='${200 + ((f * 41 + 13) % 150)}' r='${0.8 + (f % 3) * 0.5}' fill='${f % 4 === 0 ? "#ffd27a" : "#f26b1d"}' opacity='.8'/>`;
  if (w.snow)
    for (let f = 0; f < 40; f++)
      s += `<circle cx='${(f * 71 + 17) % 640}' cy='${(f * 37 + 21) % 300}' r='${1 + (f % 3) * 0.6}' fill='#fff' opacity='.55'/>`;
  s += "</svg>";
  const value = `url("data:image/svg+xml,${encodeURIComponent(s)}")`;
  cache.set(id, value);
  return value;
}
