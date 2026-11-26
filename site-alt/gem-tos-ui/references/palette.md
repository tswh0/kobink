# Palette and patterns

## The machine

| Mode | Resolution | Colours |
|---|---|---|
| ST low | 320x200 | 16 from 512 |
| ST medium | 640x200 | 4 from 512 |
| ST high | 640x400 | mono |
| STE / Falcon | up to 640x480 | 4096 (256 on screen) |

Each ST gun has 3 bits, so every channel has exactly eight levels:

```
00  24  49  6D  92  B6  DB  FF
```

That grid is the whole palette. Mixing it gives the familiar ST look: slightly chalky primaries, no true mid-tones, and no smooth blends. The Falcon's 4-bit guns add the 4096-colour range, but GEM's own chrome stayed in a handful of greys.

## UI colours

| Token | Hex | Where it comes from |
|---|---|---|
| `--gem-green` | `#00e000` | TOS 1.x desktop. Captures of 320x200 desktops sample between `#00db00` and `#00e600`; `#00e000` is the steady value. |
| `--gem-teal` | `#00aaa9` | TOS 4.04 (Falcon) desktop, sampled from a 640x480 capture. |
| `--gem-white` | `#ffffff` | window work area, fields, title text plates, menu bar. |
| `--gem-black` | `#000000` | every outline, every glyph, the menu bar text. |
| `--gem-face` | `#ffffff` / `#bababa` | button and dialog face: white in TOS 1.x, `#bababa` in TOS 4. |
| `--gem-bevel-light` | `#ffffff` | TOS 4 raised top/left. |
| `--gem-bevel-dark` | `#8a8a8a` | TOS 4 raised bottom/right. |
| `--gem-disabled` | `#8a8a8a` | disabled glyphs; pair with the 25% dither. |

TOS 1.x chrome is effectively two colours. TOS 4 adds exactly three greys and one bevel rule. Anything more is not GEM.

## The 16-colour grid

The ST could show any 16 of its 512 colours. These are the corners of the cube, useful as a fixed accent set:

| # | Hex | Name |
|---|---|---|
| 0 | `#000000` | black |
| 1 | `#242424` | 1/7 grey |
| 2 | `#494949` | 2/7 grey |
| 3 | `#6d6d6d` | 3/7 grey |
| 4 | `#929292` | 4/7 grey |
| 5 | `#b6b6b6` | 5/7 grey |
| 6 | `#dbdbdb` | 6/7 grey |
| 7 | `#ffffff` | white |
| 8 | `#ff0000` | red |
| 9 | `#00ff00` | green |
| 10 | `#0000ff` | blue |
| 11 | `#ffff00` | yellow |
| 12 | `#00ffff` | cyan |
| 13 | `#ff00ff` | magenta |
| 14 | `#db6d00` | orange |
| 15 | `#6d00db` | violet |

Use them the way GEM did: one accent, meaning only. Green = positive/OK, red = negative/stop. Everything else stays black on white.

## Dither is the only gradient

The ST had no alpha and no blending. Shading was a 1-bit pattern laid over a solid colour. Reproduce the pattern, not a grey fill:

```css
--gem-dither:    repeating-conic-gradient(#000 0 25%, #fff 0 50%) 0 0 / 2px 2px; /* 50% checker */
--gem-dither-25: radial-gradient(#000 1px, transparent 1px) 0 0 / 2px 2px;      /* 25% dots */
--gem-hatch:     repeating-linear-gradient(45deg, #000 0 1px, transparent 1px 4px);
```

| VDI pattern | Look | CSS |
|---|---|---|
| 1 | 12.5% dots | `radial-gradient(#000 1px, transparent 1px) 0 0 / 4px 4px` |
| 2 | 25% dots | the `--gem-dither-25` recipe |
| 3 | 50% checker | the `--gem-dither` recipe |
| 5 | 75% checker | invert the 25% recipe (white dots on black) |
| 9 | brick | two offset `repeating-linear-gradient` layers |
| 10 | 45 deg hatch | the `--gem-hatch` recipe |
| 11 | 45 deg cross-hatch | two hatches, one at `135deg` |
| 13 | diagonal dashes | `repeating-linear-gradient(45deg, #000 0 2px, transparent 2px 6px)` |

Where GEM used each:

- **50% checker** - active window title bar, scrollbar elevator, progress fill, pressed/selected tool.
- **25% dots** - disabled menu entries and buttons.
- **Hatch** - separators, rulers, shadowed objects, "no fill" swatches.
- **Solid inverse** - selection in lists and menus, selected icon label, active menu title.

## Theme variants

`.gem` (default) is TOS 1.x: white face, black outline, green desktop, white title bars with a dither on the active one.

`.gem--3d` is TOS 4 / Falcon: grey `#bababa` face, `#fff` top/left highlight, `#8a8a8a` bottom/right shadow, 1px black outline, teal `#00aaa9` desktop. It is the same geometry with a hard bevel - not a different design language.

Mono theme: drop the green, use black desktop or white; replace every grey with the 50% dither. This is what a 640x400 ST saw, and the fallback for high-contrast mode.

## Contrast (WCAG 2.1, calculated)

| Foreground | Background | Ratio | Verdict |
|---|---|---|---|
| `#000` | `#00e000` | 11.66:1 | pass |
| `#fff` | `#00e000` | 1.80:1 | fail - never |
| `#000` | `#00aaa9` | 7.32:1 | pass |
| `#fff` | `#00aaa9` | 2.87:1 | fail - never |
| `#000` | `#fff` | 21.0:1 | pass |
| `#000` | `#bababa` | 10.82:1 | pass |
| `#8a8a8a` | `#fff` | 3.45:1 | disabled text only |
| `#8a8a8a` | `#bababa` | 1.78:1 | fail - never |

GEM got away with grey-on-grey because a CRT blurred the pattern into a legible tone. On modern panels, keep disabled text at `#8a8a8a` only for short labels, and never as body copy.

## Colour rules from the source

- Do not decorate with colour; use it to emphasise or draw attention.
- Use colour when it carries meaning (a colour choice, a status, a document).
- When colour is a choice indicator, green is positive, red is negative.
- Never alter the user's system colours; if an app offers its own scheme, make it optional and remember it.
- Match the OS 3D effect (`objc_sysvar`) rather than inventing a second bevel style.
- Dialogs should let the user's selected dialog colour through (`FL3DBAK`), not paint their own.
