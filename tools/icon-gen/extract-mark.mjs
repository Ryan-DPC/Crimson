/**
 * Rebuilds the CRIMSONS "C" mark from the branding mock-up, then writes the two
 * source assets every icon is derived from:
 *
 *   crimson/src/assets/logos/logo_mark_only.png  -> the mark, tightly trimmed, transparent
 *   crimson/app-icon.png                         -> 1024x1024 square + margin, input for `tauri icon`
 *
 * Why this exists: the mock-up (logo_red_transparent.png) is a *render* — the red mark
 * sits on a dark textured backdrop that is only partially keyed out in its alpha channel.
 * Trimming it with a plain getbbox() keeps that ghost backdrop, which leaves the actual
 * mark tiny and off-centre in every generated icon. So we key the backdrop out ourselves
 * on chroma (the backdrop is neutral, the mark is not) and trim on the rebuilt alpha.
 *
 * Usage:  node tools/icon-gen/extract-mark.mjs
 * Then:   cd crimson && npx tauri icon
 */
import { createRequire } from 'module';
import path from 'path';
import { fileURLToPath } from 'url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const require = createRequire(path.join(ROOT, 'crimson/package.json'));
const sharp = require('sharp');

const SRC = path.join(ROOT, 'crimson/src/assets/logos/logo_red_transparent.png');
const MARK_OUT = path.join(ROOT, 'crimson/src/assets/logos/logo_mark_only.png');
const APP_ICON_OUT = path.join(ROOT, 'crimson/app-icon.png');

const CHROMA_SOLID = 45;   // above this a pixel is unambiguously part of the mark
const ALPHA_LO = 12;       // chroma -> alpha ramp, fully transparent below
const ALPHA_HI = 40;       // ... fully opaque above
const MARGIN = 0.07;       // breathing room so Windows never clips the mark
const CANVAS = 1024;

const { data, info } = await sharp(SRC).removeAlpha().raw().toBuffer({ resolveWithObject: true });
const { width: W, height: H, channels: CH } = info;
const rgbAt = (x, y) => { const i = (y * W + x) * CH; return [data[i], data[i + 1], data[i + 2]]; };
const chromaAt = (x, y) => { const [r, g, b] = rgbAt(x, y); return r - (g + b) / 2; };

// 1. Locate the mark. The wordmark under it is white, so chroma alone separates them.
let rx0 = W, ry0 = H, rx1 = -1, ry1 = -1;
for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) {
  if (chromaAt(x, y) > CHROMA_SOLID) {
    if (x < rx0) rx0 = x; if (x > rx1) rx1 = x;
    if (y < ry0) ry0 = y; if (y > ry1) ry1 = y;
  }
}
if (rx1 < 0) throw new Error(`no coloured mark found in ${SRC}`);

const slack = Math.round(Math.max(rx1 - rx0, ry1 - ry0) * 0.10);
const X0 = Math.max(0, rx0 - slack), Y0 = Math.max(0, ry0 - slack);
const X1 = Math.min(W - 1, rx1 + slack), Y1 = Math.min(H - 1, ry1 + slack);
const rw = X1 - X0 + 1, rh = Y1 - Y0 + 1;

// 2. Sample the backdrop from the neutral pixels around the mark, so edge pixels can be
//    un-composited against it instead of keeping a dark fringe.
let sr = 0, sg = 0, sb = 0, sn = 0;
for (let y = 0; y < rh; y++) for (let x = 0; x < rw; x++) {
  if (x >= 6 && y >= 6 && x < rw - 6 && y < rh - 6) continue;
  const [r, g, b] = rgbAt(X0 + x, Y0 + y);
  if (r - (g + b) / 2 < 6) { sr += r; sg += g; sb += b; sn++; }
}
const BG = sn ? [sr / sn, sg / sn, sb / sn] : [0, 0, 0];

// 3. Rebuild the alpha channel from chroma.
const keyed = Buffer.alloc(rw * rh * 4);
for (let y = 0; y < rh; y++) {
  for (let x = 0; x < rw; x++) {
    const [r, g, b] = rgbAt(X0 + x, Y0 + y);
    let a = (r - (g + b) / 2 - ALPHA_LO) / (ALPHA_HI - ALPHA_LO);
    a = a <= 0 ? 0 : a >= 1 ? 1 : a * a * (3 - 2 * a);
    const o = (y * rw + x) * 4;
    if (a <= 0) continue;
    const un = (v, bg) => a >= 0.999 ? v : Math.max(0, Math.min(255, Math.round((v - (1 - a) * bg) / a)));
    keyed[o] = un(r, BG[0]); keyed[o + 1] = un(g, BG[1]); keyed[o + 2] = un(b, BG[2]);
    keyed[o + 3] = Math.round(a * 255);
  }
}

// 4. Trim on the rebuilt alpha — this is the step the old script got wrong.
let tx0 = rw, ty0 = rh, tx1 = -1, ty1 = -1;
for (let y = 0; y < rh; y++) for (let x = 0; x < rw; x++) {
  if (keyed[(y * rw + x) * 4 + 3] > 8) {
    if (x < tx0) tx0 = x; if (x > tx1) tx1 = x;
    if (y < ty0) ty0 = y; if (y > ty1) ty1 = y;
  }
}
const tw = tx1 - tx0 + 1, th = ty1 - ty0 + 1;

const mark = await sharp(keyed, { raw: { width: rw, height: rh, channels: 4 } })
  .extract({ left: tx0, top: ty0, width: tw, height: th })
  .png({ compressionLevel: 9 }).toBuffer();
await sharp(mark).toFile(MARK_OUT);

// 5. Square it with a margin and scale to the canvas `tauri icon` expects.
const side = Math.round(Math.max(tw, th) * (1 + MARGIN * 2));
const squared = await sharp({ create: { width: side, height: side, channels: 4, background: { r: 0, g: 0, b: 0, alpha: 0 } } })
  .composite([{ input: mark, left: Math.round((side - tw) / 2), top: Math.round((side - th) / 2) }])
  .png().toBuffer();
await sharp(squared).resize(CANVAS, CANVAS, { kernel: 'lanczos3' }).png({ compressionLevel: 9 }).toFile(APP_ICON_OUT);

console.log(`backdrop keyed at rgb(${BG.map(v => Math.round(v)).join(',')})`);
console.log(`mark        ${tw}x${th}  -> ${path.relative(ROOT, MARK_OUT)}`);
console.log(`app icon    ${CANVAS}x${CANVAS} (mark fills ${(100 * Math.max(tw, th) / side).toFixed(0)}%) -> ${path.relative(ROOT, APP_ICON_OUT)}`);
console.log(`next: cd crimson && npx tauri icon`);
