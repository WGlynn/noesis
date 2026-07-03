---
name: noesis-standard
description: The Noesis house style. Swiss/International grid + editorial serif display + one pine accent, on light. Flat (no drop-shadows), hairline rules, generous whitespace, real type hierarchy. Use for every Noesis web surface (landing, deck, demo, docs). The anti-"vibe-coded" credible-infrastructure look.
---

# Noesis Standard — design system

**Intent:** read as serious infrastructure (Stripe / academic press), not a crypto casino. The
default enemy is regression-to-the-mean (glowing dark + gradient + rounded shadowed cards). We
correct it by imposing this canon: light, Swiss grid, editorial serif, ONE accent, flat.

## Palette (CSS custom properties, non-negotiable)
```css
:root{
  --bg:        #fbfbf9;  /* warm paper white */
  --bg-soft:   #f3f3ee;  /* section band */
  --panel:     #ffffff;  /* card/surface, separated by --line not shadow */
  --ink:       #17181b;  /* near-black, softer than #000 */
  --ink-dim:   #55585f;  /* secondary text */
  --ink-faint: #8b8e96;  /* captions, meta */
  --line:      #e4e3dc;  /* hairline rules + borders (1px) */
  --accent:    #0e6b5c;  /* deep pine — the ONLY accent */
  --accent-ink:#ffffff;  /* text on accent */
  --accent-soft:#e7f1ee; /* accent tint for subtle fills */
}
```

## Type
- Display / headings: **Fraunces** (Google Fonts; opsz + weight 500–900), editorial serif. Fallback Georgia, serif.
- Body / UI: **IBM Plex Sans** (weights 400/500/600). Fallback system-ui, sans-serif. (Deliberately NOT Inter — too mean-of-training.)
- Data / numbers / code: **IBM Plex Mono**. Fallback ui-monospace, monospace.
- Scale (rem, base 16px, ratio ~1.25): 0.8125 / 0.9375 / 1 / 1.25 / 1.5625 / 2.25 / 3.5.
- Body line-height 1.6; heading line-height 1.12; letter-spacing: headings -0.01em, mono +0.02em.

## Layout
- Measure (body line length): **66ch**, hard cap. Shell max-width 1120px.
- Spacing scale (px): 8 / 16 / 24 / 40 / 64 / 96. 12-col grid, 24px gutter.
- Border weight 1px (--line). Corner radius 4px max (mostly 0). **Elevation: NONE** — separate
  surfaces with hairlines + whitespace, never drop-shadows.

## Signature (keep these) / Anti-patterns (never)
KEEP: generous whitespace + strict grid · ONE pine accent, everything else ink/paper · serif
display + grotesk body + mono for all numbers/data · hairline rules instead of shadowed cards ·
the 66ch measure respected · left-aligned, not centered-everything.
NEVER: drop shadows, glow, gradient blobs, neon, rounded-everything, a second accent, dark
generic-crypto background, hero with animated particles.

## Per-surface notes
- **Landing / deck:** editorial. Big Fraunces headline, one accent, lots of air, hairline
  section dividers. Numbers (322 tests, mixes) set in mono. No decks-full-of-cards look.
- **PoA demo:** Tufte discipline. The consensus visualisation is data-ink-max, labelled, mono
  for values, one accent to mark state. PRESERVE all existing JS/wasm wiring; restyle only.
