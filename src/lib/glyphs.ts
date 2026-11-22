// Line drawings shared by the dock and the MyStyle editor, on a 24×24 grid.

export const WIFI = "M2.5 9a15 15 0 0 1 19 0M5.5 12.5a10 10 0 0 1 13 0M8.5 16a5 5 0 0 1 7 0M12 19.5h.01";
export const BLUETOOTH = "M7 7l10 10-5 4V3l5 4L7 17";
export const SPEAKER = "M4 10v4h4l5 4V6L8 10zM16 9a4 4 0 0 1 0 6";
export const POWER = "M12 3v8M7 6.5a7 7 0 1 0 10 0";

/** One rounded square of the Start logo, 7.5 wide with 1.5 corners, at (x, y). */
const tile = (x: number, y: number) =>
  `M${x + 1.5} ${y}h4.5a1.5 1.5 0 0 1 1.5 1.5v4.5a1.5 1.5 0 0 1-1.5 1.5h-4.5a1.5 1.5 0 0 1-1.5-1.5v-4.5a1.5 1.5 0 0 1 1.5-1.5z`;
/** The Start logo: four filled squares. Drawn filled, not stroked. */
export const START = tile(3.5, 3.5) + tile(13, 3.5) + tile(3.5, 13) + tile(13, 13);
