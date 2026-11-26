---
name: gem-tos-ui
description: Use when building GEM/TOS-style web UI (Atari ST look).
version: 1.1.0
author: Tom (via Hermes)
license: MIT
metadata:
  hermes:
    tags: [frontend, css, retro, atari, design-system]
    related_skills: [oat-ui, claude-design, design-md]
---

# GEM/TOS web UI

## When to Use

Any page or app that should look like Atari ST GEM/TOS: retro landing pages, demos, docs, emulator front-ends, pixel-art portfolios, "GUI as art" recreations. The user asks for GEM, TOS, Atari ST, DRI GEM, green desktop, 1px chrome, or 8x16 bitmap UI. This skill overrides the Oat default — ship `gem.css` instead of Oat, and do not mix the two on one page.

Build pages that look like Atari ST GEM/TOS: white windows with 1px black frames on a green (TOS 1.x) or teal (TOS 4.x) desktop, dithered title bars, 16px rows, an 8x16 bitmap face, and flat controls with hard bevels.

## The five rules

1. **One pixel is a design unit.** Rules are 1px. Rows are 16px. Padding is 8px. Nothing sits on a half pixel.
2. **White paper, coloured desktop, black ink.** Green on TOS 1.x, teal on TOS 4; grey chrome only in TOS 4. Colour is information, not decoration.
3. **Chrome is flat.** No radius, no shadow, no gradient, no transparency. Depth is a 1px black outline, or a hard 1px light/dark bevel in the TOS 4 theme.
4. **One typeface, one size.** Hierarchy comes from case, inverse video, underline, and space, not from font sizes.
5. **Every control is a labelled rectangle; every window has a title bar and a purpose.** If it looks interactive, it must behave like the GEM original.

## Start

```
body.gem.gem-desktop.gem--3d  1x document page, TOS 4 colours (default)
.gem.gem-screen               2x fixed desktop canvas (zoom: 2)
```

1. Copy `templates/gem.css` and `assets/atarist-8x16.woff2` into the site (same directory). `scripts/install.sh` decodes the font from `assets/atarist-8x16.woff2.b64` if needed.
2. Load `gem.css`, then put `class="gem gem-desktop gem--3d"` on `<body>` for a page, or wrap the interface in `<div class="gem gem-screen">` for a desktop canvas. Drop `gem--3d` for the classic TOS 1.x look.
3. Compose windows from `section.gem-window` (below). Start from `templates/page.html` (article) or `templates/demo.html` (desktop).

Scale rule: use `--gem-scale: 1` or `2` only. Fractional zoom makes the bitmap face blurry. At 2x every metric doubles and the 1px rules become crisp 2px device lines.

## Tokens

| Variable | Value | Use |
|---|---|---|
| `--gem-green` | `#00e000` | TOS 1.x desktop |
| `--gem-teal` | `#00aaa9` | TOS 4.x desktop (`.gem--3d`) |
| `--gem-face` | `#fff` / `#bababa` | window + button face |
| `--gem-field` | `#fff` | text fields, work area |
| `--gem-black` | `#000` | ink and every outline |
| `--gem-disabled` | `#8a8a8a` | disabled text (plus 25% dither) |
| `--gem-bevel-light` | `#fff` | TOS 4 raised highlight |
| `--gem-bevel-dark` | `#8a8a8a` | TOS 4 raised shadow |
| `--gem-dither` | 50% 1px checker | active title bars, elevators, progress |
| `--gem-dither-25` | 25% dot | disabled controls |
| `--gem-hatch` | 45 deg 1px hatch | separators, shading |

Metrics: title bar 16px + 1px rule; gadget 16x16; scrollbar 16px; button 20px tall (min 64px); field 20px; checkbox/radio 16x16; dialog padding 16px; group padding 4px 8px 8px; row gap 8px. Prose runs at 16px/24px; chrome keeps 16px rows. Keep the info line under ~28 characters (version, licence, one fact) or it ellipsizes on a 393px screen.

Themes: Tom's default is the TOS 4 / Falcon look (`gem--3d`: grey face, hard bevels, teal desktop). The TOS 1.x classic (white face, black outline, green desktop) is the alternate. Ship a checked "TOS 4 colours" checkbox in the contents panel that toggles `gem--3d` and the `theme-color` meta, so the classic green stays one click away.

## Window

```html
<section class="gem-window" aria-label="README.TXT">
  <div class="gem-titlebar">
    <button class="gem-gadget" aria-label="Close"><svg><use href="#g-closer"/></svg></button>
    <div class="gem-title"><span>README.TXT</span></div>
    <button class="gem-gadget gem-gadget--right" aria-label="Full size"><svg><use href="#g-fuller"/></svg></button>
  </div>
  <div class="gem-infoline">1.4 KB used - 640 x 400 - 16 colours</div>
  <div class="gem-body">...</div>
</section>
```

- Title text sits on a white plate so the dither never runs through the glyphs.
- Active window: dithered title bar. Inactive: add `gem-window--inactive` (plain white bar).
- The info line is optional but idiomatic: file size, selection, zoom, byte count.
- Title = filename, `Untitled`, or the task ("Find & Replace"). Duplicate views use `NAME:1`, `NAME:2` only when more than one exists.
- A window with a closer is modeless; a dialog without one is modal. Don't mix the signals.
- On a static page omit gadgets entirely rather than ship a closer or fuller that does nothing; the title bar and dither carry the look on their own.
- For a scrolling document use the `.gem-scroll` grid (view + 16px arrows/track/elevator + sizer); markup in `references/components.md`.

## Menu bar

```html
<nav class="gem-menubar" aria-label="Main">
  <div class="gem-menuwrap"><button class="gem-appname" data-menu="m-desk">Desk</button>
    <ul class="gem-dropdown" id="m-desk" role="menu">...</ul>
  </div>
  <div class="gem-menuwrap"><button data-menu="m-file">File</button>...</div>
  <span class="gem-spacer"></span>
  <div class="gem-menuwrap"><button data-menu="m-help">Help</button>...</div>
</nav>
```

Order: application name (uppercase), `File`, `Edit`, application menus, `Help` last (`Window` second-last if present). Seven titles maximum. Ellipsis means "opens a dialog"; no ellipsis means the action is immediate. Disabled items are greyed with the 25% dither, not hidden. Checkmarks or swapped labels show state. Nested submenus are legal but discouraged. Give the rightmost menu `.gem-dropdown--right` (`left:auto; right:0`), or its 160px panel opens off the right edge. The 20-line menu script sits at the bottom of `templates/demo.html`; keep Escape and outside-click closing.

## Buttons, fields, choices

```html
<button class="gem-btn is-default"><u>O</u>K</button>
<button class="gem-btn"><u>C</u>ancel</button>
<button class="gem-btn is-disabled">Save</button>

<fieldset class="gem-group"><legend>Display</legend>
  <div class="gem-check-row"><input class="gem-check" type="checkbox" id="c1" checked><label for="c1">B&amp;W desktop</label></div>
</fieldset>

<select class="gem-select">...</select>
<input class="gem-field" type="text">
```

- Button order: `OK`, `Cancel`, other actions, `Help`. `OK` in capitals, everything else capitalised. Positive before negative (`Yes` before `No`).
- The default button gets the doubled outline (`.is-default`). Make the negative action the default only when the action is irreversible.
- Underline the keyboard accelerator letter with `<u>`; wire the real key in JS.
- Pressed controls are inverted or dithered (`:active`, `.is-pressed`); disabled use grey + dither.
- Checkbox: square, X when checked. Radio: circle, dot when checked. No rounded corners, no smooth ticks.

## Dialog and alert

```html
<section class="gem-dialog" role="dialog" aria-modal="true" aria-labelledby="dlg-title">
  <div class="gem-titlebar">
    <button class="gem-gadget" aria-label="Close">...</button>
    <div class="gem-title"><span id="dlg-title">User interface</span></div>
  </div>
  <div class="gem-dialog__body">...</div>
  <div class="gem-dialog__buttons">
    <button class="gem-btn is-default"><u>O</u>K</button>
    <button class="gem-btn"><u>C</u>ancel</button>
    <button class="gem-btn"><u>H</u>elp</button>
  </div>
</section>
```

Alert body: a 32x32 black icon square (`?`, `!`, `i`, stop) and left-aligned text, ~30 characters per line, 5 lines max. Say what happened and what to do; no "error", no blame, use "Cannot". Never stack alerts.

## Progress and feedback

```html
<div class="gem-progress" role="progressbar" aria-valuenow="62" aria-valuemin="0" aria-valuemax="100">
  <div class="gem-progress__fill" style="width:62%"></div>
</div>
```

Fill with the 50% dither, 1px black frame, optional `0% / 25% / 50% / 75% / 100%` labels underneath. Progress must be proportional to real time; put long tasks in a window so the user can keep working; offer abort (GEM used ESC). Prefer a bar to a percentage alone.

## Composition patterns

- **Desktop canvas** (`templates/demo.html`): menu bar at the top, overlapping windows on the green field, icons with white labels, one dialog and one alert. Use 2x.
- **Document page** (`templates/page.html`): menu bar, a 640px document window plus a 224px contents window, a status line footer. Use 1x. The pattern for blogs, docs, changelogs, install pages. Prose is 16/24; code blocks are inverse plates (black fill, white text, 1px border; `white-space: pre-wrap` for short configs, `pre` + `overflow-x: auto` when lines are long); inline code is a small inverse chip; the contents window is sticky; if you give it a closer, wire it to hide the window and a menu or status-bar link to restore it; a group-box "System" panel carries the facts; a checkbox toggles `.gem--3d` for a live TOS 4 variant. Markup and CSS in `references/components.md`.
- **Rebuild an existing site**: scrape the live pages, keep the content and URLs, and regenerate with the document-page pattern. Procedure, extraction rules and the pitfalls that cost a round in `references/rebuilding-a-site.md`.
- **App screen**: one window per task, menu bar always present, dialogs only for modal decisions, info line for selection state.
- **Landing**: the window is the hero; keep one primary window and one dialog, don't scatter cards.

Mobile: stack windows (flex column), drop secondary menu titles, keep 32px minimum tap targets, never shrink the font below 16px.

## Behaviour and copy (from the GEM guidelines)

- Menus: app name first; `File`, `Edit`; `Help` rightmost; 7 titles max; group with separators; ellipsis opens a dialog; grey out what cannot run now.
- Dialogs: confirm with `OK`, revert with `Cancel`, offer `Help` whenever help exists. Button order `OK, Cancel, ..., Help`, centred at the bottom or in a top-right column. Default = positive, except destructive/irreversible actions. Advanced options use `Options >>` / `<< Options`.
- Alerts: more than one option when possible; never `OK` + `Cancel` when both do the same; no "error", no accusation; `Cannot` not `Can't`; a remedy in the text; left-aligned; 5 lines x 30 chars max; no consecutive alerts.
- Colour: never decorate; emphasise. Green = positive, red = negative. Never alter the user's system colours.
- Sound: the system bell only, as a reminder or completion signal.
- Keyboard: `^N ^O ^S ^P ^Q ^X ^C ^V ^A ^F ^R`; Help key; Shift-click extends selection; shortcuts flush right in menus; don't duplicate a shortcut.
- Localisation: allow +50% text width; no words baked into graphics; keep dates/times/currency out of the layout.
- Custom controls must behave like the standard ones; if an enhancement departs from the standard, let the user turn it off.

## Web realities

- **Semantics first**: `nav`, `section`, `dialog`, `fieldset/legend`, `label`, real `button`. The CSS assumes them.
- **Focus**: visible dotted 1px outline, offset 2px. GEM had no focus ring; the web needs one. Black on white, white on black.
- **Contrast**: black on `#00e000` is 11.7:1; white on green is 1.8:1 - never put white text on the desktop. `#8a8a8a` on white is 3.5:1, so use it for disabled states only, not body text.
- **Geometry**: integers only. `border-radius: 0`, no `box-shadow` except the 1px inset bevel, no `linear-gradient` except dithers, no transitions (the only animation is the 1-block caret; respect `prefers-reduced-motion`).
- **Icons**: SVG on a 16 or 32 grid, `shape-rendering: crispEdges`, 1px black outline, one fill. `image-rendering: pixelated` when scaling.
- **Font**: WOFF2 is ~6.5 KB. Set `font-size: 16px; line-height: 16px`; do not synthesise bold/italic - use uppercase, inverse, or underline. If you need real bold, ship a second bitmap face. The face is ASCII (plus Latin-1/Cyrillic): smart quotes, em dashes, arrows and box drawing are missing. Normalise them in the build (curly quotes to straight quotes, em dash to `-`, arrows to `->`/`<-`, box drawing to `+`/`|`/`-`) or the browser falls back mid-line; Safari can carry that fallback through the rest of the run, usually the text after a link or code chip. `scripts/check-glyphs.py` lists the offenders.
- **Scrolling**: hide native scrollbars (`scrollbar-width: none`) and use `.gem-scroll` for the visible GEM bars, or style `::-webkit-scrollbar` to 16px blocks. Keep wheel/keyboard scrolling on the real scroll container.

## Pitfalls

- Sharp 1px lines need integer positions; avoid centring with `transform: translate(-50%)` (half pixels).
- `zoom` must be an integer; `1.5` blurs the face and the rules.
- Don't use the green as a large text background in long documents; it is a desktop, not a page colour. Put content on white.
- Don't fake the dither with a PNG; CSS patterns stay sharp at every scale.
- Don't add drop shadows to windows; depth is stacking order + black outlines.
- Don't mix TOS 1.x (white/flat) and TOS 4 (grey/bevel) chrome in one screen.
- Avoid `font-weight: bold` on the bitmap face: browser synthesis smears pixels. Use uppercase or inverse video.
- `code`, `kbd`, `samp` keep the UA monospace unless you set `font: inherit`; a font switch inside a line looks wrong and some browsers carry the fallback into the following text.
- A glyph the face does not have (curly quote, em dash, arrow) does the same thing: the browser falls back, and the run after a link or code chip can switch font for the rest of the line. Normalise punctuation and check the build.
- The 8x16 face has no emoji or dingbats: `✨` renders as a tofu box. Replace it with ASCII punctuation, or wrap it in a span with the system emoji font when the emoji is content.
- Pad inline code chips by one full cell (8px); Atari glyphs sit flush in their cell, so 4px padding reads as touching the next word.

## Verify

1. Screenshot at 1x and 2x; every rule is exactly 1px (2px at 2x) and nothing lands between pixels.
2. Tab through: focus always visible; menus open/close with keyboard; Escape closes.
3. Contrast: black on white/green/teal only; no white on green; no grey body text.
4. Resize to 360px: windows stack, menu collapses, no horizontal scroll. Verify the mobile layout with a Playwright viewport (or a real device), not `chrome --headless --window-size=393,852`: desktop Chrome clamps the layout viewport, the media query never fires, and the shot looks broken while the CSS is fine.
5. Run the copy through the rules: button order, ellipses, no "error", sentence case.
6. Check a paragraph with inline code and the rightmost menu at both scales: code must use the 8x16 face, and the menu must open leftwards inside the viewport.
7. Run `scripts/check-glyphs.py <site-dir>`; it must print `ok`. A missing glyph breaks the face mid-run.

## Files

- `templates/gem.css` - complete stylesheet (tokens, window, menu, forms, dialog, alert, progress, scroll, icons, patterns).
- `templates/demo.html` - 640x480 desktop: menu bar, document window with scrollbars, settings dialog, alert, icons, gadget sprite.
- `templates/page.html` - 1x article page: menu bar, document + contents windows, progress, footer.
- `references/palette.md` - colour system, ST 16-colour grid, dither recipes, contrast table.
- `references/components.md` - full markup and state notes for every component.
- `references/guidelines.md` - the source rules, organised by topic, with citations.
- `references/rebuilding-a-site.md` - converting an existing site (HTML, Hugo, a static generator) into this system without losing content or URLs.
- `assets/atarist-8x16.woff2` (+ `.b64`) - the 8x16 system face.
- `scripts/install.sh` - decode the font; `scripts/bdf2woff.py` - rebuild from the BDF; `scripts/check-glyphs.py` - find characters the face cannot render.

## Sources

The ATARI Compendium, chapter 11, *GEM User Interface Guidelines* (mirror: aresluna.org/ui-style-guides/GEM and TOS/GEM User Interface Guidelines/770.html); *The documentation for TOS*, 10.2 *Style guidelines* (same collection); Atari TOS 1.x and 4.04 screenshots for the palette and chrome. The font is the Atari ST 8x16 system face, converted from ntwk/atarist-font (BSD-3-Clause); see `assets/FONT-LICENSE.txt`.
