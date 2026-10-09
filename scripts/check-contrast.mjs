import { readFileSync } from 'node:fs';
import assert from 'node:assert/strict';
const css = readFileSync(new URL('../apps/desktop/src/styles.css', import.meta.url), 'utf8');
function luminance(hex) {
  let value = hex.slice(1);
  if (value.length === 3) value = [...value].map(c => c + c).join('');
  return [0.2126, 0.7152, 0.0722].reduce((sum, weight, i) => {
    const channel = parseInt(value.slice(i * 2, i * 2 + 2), 16) / 255;
    return sum + weight * (channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4);
  }, 0);
}
for (const theme of ['light', 'dark']) {
  const block = theme === 'light' ? css.split('}')[0] : css.split(":root[data-theme='dark'] {")[1].split('}')[0];
  const colors = Object.fromEntries([...block.matchAll(/--([\w-]+): (#[a-f0-9]+);/g)].map(m => [m[1], m[2]]));
  for (const foreground of ['text-primary', 'text-secondary', 'text-muted', 'accent', 'warning', 'danger', 'success', 'information']) {
    for (const background of ['canvas', 'surface', 'surface-raised', 'surface-muted', 'selection']) {
      const a = luminance(colors[foreground]), b = luminance(colors[background]);
      assert((Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05) >= 4.5, `${theme}: ${foreground} on ${background}`);
    }
  }
}
console.log('Contrast: 80 text/background pairs pass 4.5:1.');
