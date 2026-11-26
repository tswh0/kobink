# site-alt — the GEM/TOS edition

An alternate install page, drawn in the interface language of the Atari ST:
1px black chrome, dithered title bars, an 8x16 bitmap face, and a desktop in
TOS 4 teal by default (untick **TOS 4 colours** in Contents for the classic
TOS 1.x green).

It is self-contained: `index.html`, `page.css`, `gem.css`, the WOFF2 face and
the favicons. Open `index.html` directly, or:

```sh
python3 -m http.server 8123 --directory site-alt
```

This folder is not in the deploy path: `site/` is still what Caddy serves at
kobink.ewlf.de.

## The skill

The system behind this page ships in [`gem-tos-ui/`](gem-tos-ui/): the skill
document, its references, the stylesheet, the 8x16 face, the glyph checker
and the two page templates. Copy that folder into a site to build another
page in the same language.

## Licences

- `gem.css` and the page structure come from the `gem-tos-ui` skill (MIT).
- `atarist-8x16.woff2` is a conversion of the Atari ST 8x16 system font
  ([ntwk/atarist-font](https://github.com/ntwk/atarist-font), BSD-3-Clause);
  the notice is in `FONT-LICENSE.txt` and `gem-tos-ui/assets/FONT-LICENSE.txt`.
- The page copy is AGPL-3.0 like the rest of the repository.
