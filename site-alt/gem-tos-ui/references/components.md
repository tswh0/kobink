# Components

All markup assumes `gem.css` is loaded and the page is inside `.gem`. The gadget sprite used below lives at the top of `templates/demo.html`; copy it once per page.

## Window

```html
<section class="gem-window" aria-label="README.TXT">
  <div class="gem-titlebar">
    <button class="gem-gadget" aria-label="Close"><svg><use href="#g-closer"/></svg></button>
    <div class="gem-title"><span>README.TXT</span></div>
    <button class="gem-gadget gem-gadget--right" aria-label="Iconify"><svg><use href="#g-iconify"/></svg></button>
    <button class="gem-gadget gem-gadget--right" aria-label="Full size"><svg><use href="#g-fuller"/></svg></button>
  </div>
  <div class="gem-infoline">1.4 KB used - 640 x 400 - 16 colours</div>
  <div class="gem-body">...</div>
</section>
```

Anatomy, in order: 1px outer border, title bar (16px) with gadgets, optional info line (16px + 1px rule), work area, optional scrollbars, sizer.

- `.gem-window--inactive` turns the title bar white.
- `.gem-gadget` is 16x16 with a 1px divider; the glyph is an 8x8 pixel SVG.
- Title text is centred on a white plate: `.gem-title span { background:#fff; padding:0 4px; }`.
- The info line is for size, selection, zoom, free space - context, not decoration.

### Scrollable window

```html
<div class="gem-scroll">
  <div class="gem-scroll__view"><div class="gem-body">...</div></div>
  <div class="gem-scroll__v">
    <button class="gem-scroll__arrow gem-scroll__arrow--up" aria-label="Up"></button>
    <div class="gem-scroll__track"><div class="gem-scroll__elevator"></div></div>
    <button class="gem-scroll__arrow gem-scroll__arrow--down" aria-label="Down"></button>
  </div>
  <div class="gem-scroll__h">
    <button class="gem-scroll__arrow gem-scroll__arrow--left" aria-label="Left"></button>
    <div class="gem-scroll__track"><div class="gem-scroll__elevator"></div></div>
    <button class="gem-scroll__arrow gem-scroll__arrow--right" aria-label="Right"></button>
  </div>
  <div class="gem-sizer"></div>
</div>
```

- Column 2 / row 2 are 16px; the sizer sits in the corner cell.
- The elevator is a 50% dither block with a 1px border; the track is white.
- Keep the real scroller on `.gem-scroll__view`: wheel, keyboard, and touch must work. The visible bars are decorative; wire the arrows with JS if the page needs them, or leave them as affordances.
- For vertical-only windows add `.gem-scroll--v` and omit the `.gem-scroll__h` element: the view spans both rows and the sizer keeps the bottom-right corner.

## Menu bar

```html
<nav class="gem-menubar" aria-label="Main">
  <div class="gem-menuwrap"><button class="gem-appname" data-menu="m-desk" aria-haspopup="true" aria-expanded="false">Desk</button>
    <ul class="gem-dropdown" id="m-desk" role="menu">
      <li><a href="#">GEM Desktop</a></li>
      <li class="gem-sep" role="separator"></li>
      <li><a href="#" class="is-disabled" aria-disabled="true">Control Panel</a></li>
    </ul>
  </div>
  <span class="gem-spacer"></span>
  <div class="gem-menuwrap"><button data-menu="m-help">Help</button>...</div>
</nav>
```

- `.gem-appname` uppercases the first title; it is the program name, not "Desk", on MultiTOS and later.
- `.gem-spacer` pushes Help to the right.
- `.gem-dropdown` is absolutely positioned in the wrapper; `top:16px` sits it under the bar. Add `.gem-dropdown--right` to the rightmost menu so it opens leftwards instead of off-screen.
- Items are 16px rows; shortcuts use `<span class="gem-key">^S</span>` and align right.
- `.gem-sep` is a 1px rule with 2px margins.
- Disabled: `.is-disabled` adds the 25% dither and grey text.
- The open state is `.is-open` on both the dropdown and its button.

Menu script (keep it small; no framework):

```js
const closeMenus = () => {
  document.querySelectorAll('.gem-dropdown.is-open').forEach(m => m.classList.remove('is-open'));
  document.querySelectorAll('[data-menu][aria-expanded="true"]').forEach(b => { b.classList.remove('is-open'); b.setAttribute('aria-expanded', 'false'); });
};
document.querySelectorAll('[data-menu]').forEach(btn => {
  btn.addEventListener('click', event => {
    event.stopPropagation();
    const menu = document.getElementById(btn.dataset.menu);
    const wasOpen = menu.classList.contains('is-open');
    closeMenus();
    if (!wasOpen) { menu.classList.add('is-open'); btn.classList.add('is-open'); btn.setAttribute('aria-expanded', 'true'); }
  });
});
document.addEventListener('click', closeMenus);
document.addEventListener('keydown', event => { if (event.key === 'Escape') closeMenus(); });
```

For a production site add arrow-key roving focus inside the open menu and return focus to the button on close.

## Buttons

```html
<button class="gem-btn is-default"><u>O</u>K</button>
<button class="gem-btn"><u>C</u>ancel</button>
<button class="gem-btn" disabled>Save</button>
<button class="gem-btn is-pressed" aria-pressed="true">Bold</button>
```

- Base: 20px tall, min 64px, 1px black border, face colour, centred text.
- `.is-default`: 2px `double` border - the GEM "default exit" mark. Only one per dialog.
- `:active` / `.is-pressed`: 50% dither fill; in `.gem--3d` the bevel inverts and the label shifts 1px.
- `:disabled` / `.is-disabled`: grey text + 25% dither; still focusable is wrong - use the real `disabled` attribute.
- Accelerator: underline the letter with `<u>`; GEM underlined the key, not the whole label.

## Fields and choices

```html
<div class="gem-form">
  <label for="name">Name:</label>
  <input class="gem-field" id="name" type="text" value="README.TXT">
  <label for="kind">Type:</label>
  <select class="gem-select" id="kind"><option>Document</option><option>Folder</option></select>
</div>

<fieldset class="gem-group">
  <legend>Display</legend>
  <div class="gem-check-row"><input class="gem-check" type="checkbox" id="bw" checked><label for="bw">B&amp;W desktop</label></div>
  <div class="gem-radio-row">
    <input class="gem-radio" type="radio" name="r" id="r1" checked><label for="r1">Button</label>
    <input class="gem-radio" type="radio" name="r" id="r2"><label for="r2">Wedge</label>
  </div>
</fieldset>
```

- `.gem-form` is a two-column grid: right-aligned labels, flexible controls, 8px gap.
- Fields: 20px tall, white, 1px black border; focus is a 1px dotted outline offset 1px.
- `.gem--3d` fields are sunken with the bevel reversed.
- Checkbox is a square with an X (two 45 degree gradients); radio is a circle with a centred dot. Both are 16x16 with a 1px black border.
- `.gem-group` is a fieldset with a 1px frame; the legend sits in the top rule with 4px side padding.
- Selects use the pixel arrow via `background-image` and 20px right padding.

Textarea: `.gem-textarea` (auto height, 4px padding, resize both).

## Dialog

```html
<section class="gem-dialog" role="dialog" aria-modal="true" aria-labelledby="dlg-title">
  <div class="gem-titlebar">
    <button class="gem-gadget" aria-label="Close"><svg><use href="#g-closer"/></svg></button>
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

- `.gem-dialog` is a grey (3D) or white (classic) panel with a 1px border and a title bar; no full-size gadget.
- Body padding 16px; button row centred with 16px gaps, 16px bottom padding.
- Buttons: OK, Cancel, other actions, Help. Positive before negative.
- A closer means modeless; omit it for a modal dialog and add a focus trap.
- "Unfolding" advanced options: an `Options >>` button that becomes `<< Options`.

## Alert

```html
<div class="gem-alert">
  <div class="gem-alert__icon">?</div>
  <div class="gem-alert__text">The printer does not reply. Check that it is switched on and connected.</div>
</div>
```

- Icon square is 32x32, black, white glyph: `?` question, `!` warning, `i` information, a stop sign for fatal, a disk for disk errors.
- Text: left-aligned, max 30 characters per line, 5 lines.
- Icons are semantic, not decorative: question for a choice, exclamation for attention, stop for irreversible.

## Progress

```html
<div class="gem-progress" role="progressbar" aria-valuenow="62" aria-valuemin="0" aria-valuemax="100">
  <div class="gem-progress__fill" style="width:62%"></div>
</div>
<div class="gem-progress__labels"><span>0%</span><span>25%</span><span>50%</span><span>75%</span><span>100%</span></div>
```

- 16px tall, 1px frame, 1px inner padding, 50% dither fill, 1px right edge on the fill.
- Labels are optional; GEM's thermometer had them under the bar.
- Use a real `<progress>` element if you prefer native semantics and style it the same way.

## Toolbar and toolbox

```html
<div class="gem-toolbar" role="toolbar" aria-label="Editor">
  <button class="gem-tool" aria-label="Open"><svg><use href="#g-folder"/></svg></button>
  <button class="gem-tool" aria-label="Save"><svg><use href="#g-floppy"/></svg></button>
  <span class="gem-sep-v"></span>
  <button class="gem-tool is-pressed" aria-label="Bold"><b>B</b></button>
</div>
```

- `.gem-toolbar` is a single row under the title/info line, 4px padding, 4px gaps, vertical 1px separators.
- `.gem-tool` is 20x20 with a 1px border; `.is-pressed` dithers.
- `.gem-toolbox` is a two-column grid for a floating palette.
- Toolbar icons: 16px art on a 16 grid, black outline, white fill. Keep one visual weight.

## Desktop icons

```html
<a class="gem-icon" href="#"><svg><use href="#g-floppy"/></svg><span class="gem-icon__label">FLOPPY DISK</span></a>
<a class="gem-icon is-selected" href="#"><svg><use href="#g-folder"/></svg><span class="gem-icon__label">DOCUMENTS</span></a>
```

- 32x32 pixel art, label on a white plate, 2px gap.
- Selected: label inverts (black plate, white text); single click selects, double click opens.
- Icons are 2x-scaled 16px grids in the template; keep `shape-rendering="crispEdges"`.

## Status line

```html
<footer class="gem-status">
  <span>dev adjacent</span><span>640 x 400</span><span class="gem-grow"></span><span>16 colours</span>
</footer>
```

- 1px top rule, 4px padding, 16px gaps, white or face background.
- Use it for document state, selection count, zoom, connection - not navigation.

## Prose, code and the document page

A 1x document page (`templates/page.html` is the starting point) puts one 640-720px `.gem-window` beside a 224-240px contents window, both on the desktop (green on TOS 1.x, teal on TOS 4).

- Body copy: 16px font, 24px line-height. This is the one place a second leading is allowed; UI rows keep 16px.
- `h2` is uppercase with a 1px bottom rule and 32px of space above. `h3` is underlined. Both stay 16px.
- Lists are flat: `list-style:none`, a `· ` pseudo-element, 24px rows.

Code blocks are inverse plates, which reads as a GEM dialog field and keeps long commands legible:

```css
.doc pre {
  margin: 0 0 16px;
  padding: 8px;
  background: #000;
  color: #fff;
  border: 1px solid #000;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  line-height: 20px;
}
.doc p code, .doc li code { background: #000; color: #fff; padding: 0 8px; }
```

Short config blocks (compose files, a few commands) use this `pre-wrap` form so a 393px screen does not send them sideways. A document with long shell lines should override it with `white-space: pre; overflow-x: auto; overflow-wrap: normal;` — wrapping them produces ragged blocks that are worse than a scrollbar. Inline `<code>` uses the same inverse chip, padded by one full cell (8px) so it does not touch the next word.

The contents window:

```html
<aside class="page__side" id="contents">
  <section class="gem-window">
    <div class="gem-titlebar">
      <button class="gem-gadget" aria-label="Close contents" data-close-contents><svg><use href="#g-closer"/></svg></button>
      <div class="gem-title"><span>CONTENTS</span></div>
    </div>
    <div class="gem-body">
      <ul class="side-list"><li><a href="#install">Install</a></li>...</ul>
      <fieldset class="gem-group"><legend>System</legend>
        <dl class="sys"><dt>Image</dt><dd>ghcr.io/owner/app:latest</dd>...</dl>
      </fieldset>
      <div class="gem-check-row"><input class="gem-check" type="checkbox" id="tos4" checked><label for="tos4">TOS 4 colours</label></div>
    </div>
  </section>
</aside>
```

- `position: sticky; top: 24px` on the aside; it stacks below the document under about 800px.
- The closer hides the window (`contents.hidden = true`); a menu item and a status-bar link with `data-open-contents` restore it. Never leave a closer that does nothing.
- When the aside is hidden, `:has(.page__side[hidden])` can re-centre the document; cap the document at 720px so closing the panel does not produce 90-character lines.
- A group-box "System" panel is the GEM way to carry reference facts (image, port, licence) without a second page.
- The TOS 4 checkbox is checked by default and is the cheapest live demo of the theme: `body.classList.toggle('gem--3d', checked)`.

## Patterns and utilities

| Class | Effect |
|---|---|
| `.gem-dither` | 50% checker |
| `.gem-dither-25` | 25% dot |
| `.gem-hatch` | 45 degree hatch |
| `.gem-invert` | black plate, white text |
| `.gem-nowrap` | no wrapping |
| `.gem-grow` | `flex:1` |
| `.gem-mono` | force the bitmap face |

## Gadget sprite

The three window glyphs and the desktop icons are inline SVG paths in `templates/demo.html`. To reuse: copy the hidden `<svg>` block once, then reference `#g-closer`, `#g-fuller`, `#g-iconify`, `#g-floppy`, `#g-folder`, `#g-trash`. They are 32x32 viewBoxes drawn on a 2px pixel grid; render them at 8px (gadgets), 16px (tools), or 32px (desktop).
