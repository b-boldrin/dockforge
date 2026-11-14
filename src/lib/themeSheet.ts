// Themes are applied as constructed stylesheets instead of <style> tags. That keeps the
// Content-Security-Policy strict (no inline styles) while letting a theme change instantly.
// Any url() in a theme was already rewritten by the Rust side into a data: URL of a file
// inside the theme folder.

export function createSheet(): CSSStyleSheet {
  const sheet = new CSSStyleSheet();
  document.adoptedStyleSheets = [...document.adoptedStyleSheets, sheet];
  return sheet;
}

export function setSheet(sheet: CSSStyleSheet, css: string, label: string) {
  try {
    sheet.replaceSync(css);
  } catch (err) {
    console.error(`Theme ${label} could not be applied`, err);
    sheet.replaceSync("");
  }
}

/** Limits a theme to one element (a preview in the settings window), so its rules can never
 *  restyle the rest of the window. */
export function scoped(css: string, selector: string) {
  return `@scope (${selector}) {\n${css}\n}`;
}
