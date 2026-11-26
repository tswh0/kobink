# Source guidelines

The rules below are distilled from two period documents, both mirrored in Marcin Wichary's UI style-guide collection:

- **The ATARI Compendium, chapter 11 - GEM User Interface Guidelines** (nodes 770-791). Chapter index: `https://aresluna.org/ui-style-guides/GEM%20and%20TOS/GEM%20User%20Interface%20Guidelines/770.html`
- **The documentation for TOS, 10.2 - Style guidelines**: `https://aresluna.org/ui-style-guides/GEM%20and%20TOS/The%20documentation%20for%20TOS/guidelines_styles.html`

Node numbers below refer to the first document; "TOS 10.2" refers to the second. The originals are written for C/AES programmers; each section ends with how to read it for the web.

## 1. Why consistency (770, 771)

Maintaining consistent elements of style is not cosmetic: an otherwise powerful application is compromised by an interface unlike the majority of others the user already knows. A program should look and behave like the rest of the system; if it offers an enhanced element that departs from the specification, it must at least let the user disable it in a Settings dialog.

Applications are expected to have a menu bar. "Dialogware" and "Alertware" - apps that live entirely in one dialog or alert - are explicitly discouraged because they stop the user from using other programs.

**Web:** a site or app is one interface among many. Keep the chrome conventional even when the content is unusual. Novel interactions need an off switch or a conventional alternative.

## 2. Windows (772)

- A window is a viewport on a document; windows are modeless. Opening one never blocks the rest of the application.
- A normal document window has a title bar and is movable. Provide sliders when content can overflow, and keep them proportional to the visible/total ratio.
- Include all standard elements; only omit one when it would be inappropriate in context (the info line is the usual omission).
- **Title:** the filename, or `Untitled` for a new document. A non-document window is titled by its purpose ("Find & Replace"). Duplicate views of one document are `NAME:1`, `NAME:2`, and the number appears only when a second view exists.
- **Info line:** context that changes with the selection - size, statistics, zoom, number of selected objects.
- **Iconify:** a single window becomes an icon; iconifying again adds it to a "program group"; uniconifying a group restores all its windows. Draw the content for a single icon and the application for a group.
- **Colours:** an application may colour its own windows but must never change the global window colours; the user controls those with the Window Colors CPX.

**Web:** think panels and workspaces, not full-screen takeovers. A route is a window: it has a title (document title / h1), an optional status line, and it never traps the user. Tabs are duplicate views; number them only when there is more than one.

## 3. Dialogs (774)

- A dialog is the modal counterpart to a window: input goes to it until the user satisfies it.
- If the dialog controls a physical attribute (text face, fill type), show a live sample area before the user exits.
- Auto-centring on the window or pointer is convenient for some and annoying for others; make it optional.
- Dialogs that supply information: `OK`, plus `Help` if help exists. Dialogs that change settings: `OK`, `Cancel`, plus `Help`.
- **Button order:** `OK`, `Cancel`, other buttons, `Help` (left to right or top to bottom). `OK` is all capitals; all other buttons are capitalised. When the wording is `Yes/No`, the positive answer comes first.
- Every dialog has a default exit button. Normally it is the positive response. When the action is irreversible and changes data (formatting a disk), the negative response should be the default.
- **Placement:** either centred at the bottom, or a column down the upper-right corner. In the column style, one blank row may group complementary buttons.
- **Unfolding dialogs:** advanced options hide behind `Options >>` / `More >>`; when open, the button becomes `<< Options` / `<< Less` and returns the dialog to its first state.
- Custom controls must behave like the standard object they imitate (a custom text control responds to keys like `G_FTEXT`); if they depart, allow disabling them.

**Web:** one primary action, one escape. Footer order left-to-right: OK, Cancel, extras, Help. Use `dialog` semantics and a focus trap only for real decisions; everything else is a panel. The destructive default rule survives: don't focus "Delete".

## 4. Alerts (775)

- Apply the dialog button rules.
- Offer more than one option whenever possible; a one-button alert is frustrating and only justified when there is exactly one possible action.
- Never show `OK` and `Cancel` when both lead to the same outcome.
- Avoid the word "error" and any text that blames the user.
- If something failed, suggest a remedy - possibly a dialog for re-entry.
- Use `Cannot`, not `Can't` or `Can not`.
- When the alert may appear while another process has focus, prefix the first line with the program name and a colon.
- Be specific: "Not enough memory to load file TEST.DOC." beats "Insufficient memory."
- Escalate gently: first a bell, then a dialog that guides the user.
- Message text is left-aligned.
- If the message does not fit 5 lines of 30 characters, shorten it or put it in a form; never use consecutive alerts.
- Capitalise by normal grammar and end with a period or question mark - not an exclamation mark.
- Icons: 0 none (credits, reminders), 1 stop/error requiring immediate action, 2 question (inquiries, most confirmations), 3 exclamation (fatal errors, irreversible confirmations), 4 information (usually OK only), 5 disk error.

**Web:** this is a copywriting spec, not just a layout. Toasts and inline errors should name the file/object, state the remedy, and avoid "Oops" and "Something went wrong".

## 5. Menus (780)

- Every application initialises a menu bar. The leftmost title is the application name (formerly "Desk" or the fuji logo).
- First item of that menu is `PRGNAME...`; the lines below it are desk accessories and other applications.
- Second and third titles: `File`, `Edit`. Application menus follow. `Help` is rightmost; `Window` is second-rightmost.
- Entries are grouped by function and subgrouped with separator bars.
- An ellipsis means the entry opens a dialog; no ellipsis means it acts immediately.
- **File menu order:** New, Open..., Recall (optional), Save, Save as..., Save all (optional), closing commands; separator; Import, Export; separator; Page Setup..., Print; separator; Quit.
- **Edit menu order:** Undo, Redo; separator; Cut, Copy, Paste, Delete; separator; Select All (optional); separator; Find..., Replace..., Find Next; separator; other editing commands.
- Dual-state items either take a checkmark or swap their label (`Hide Toolbar` becomes `Show Toolbar`). Groups with more than two states use a checkmark.
- Preferences belong in the `Options` menu. A `Window` menu can offer New Window, Arrange/Tile/Cascade, then a cascading list of open windows.
- A `Help` menu offers levels (Contents, Index), not one item per dialog; context help belongs on a Help button or the Help key.
- Popup menus appear at the pointer (usually right-click). Objects that open a popup are TOUCHEXIT and SHADOWED; a text label to the left inverts while the popup is open. The current option is passed to `menu_popup` so the popup aligns to it, in addition to its checkmark. A popup with no selection starts at the first item. Popups may contain non-text objects but cannot scroll.
- **Drop-down list boxes** follow popup rules, except: no checkmark next to the current selection, and an "equivalence" character (ASCII 240) sits immediately right of the box, clickable like the box. The box may be editable to add items.
- Nested submenus are possible but discouraged - the bar should never be that complex.

**Web:** `nav` with the product name first, then File-like tasks, Help last. Cap the top level at seven. Use a chevron or ellipsis consistently for "opens more". Don't nest menus more than one level. Keep the same order on every page.

## 6. Keyboard (784, TOS 10.2.8)

Universal equivalents (enable wherever the counterpart exists):

| Keys | Action |
|---|---|
| Ctrl-N | New |
| Ctrl-O | Open |
| Ctrl-W | Close |
| Ctrl-S | Save as... |
| Ctrl-Shift-S | Save |
| Ctrl-P | Print |
| Ctrl-Shift-P | Page Setup |
| Ctrl-Q | Quit |
| Ctrl-X / C / V | Cut / Copy / Paste |
| Ctrl-A | Select all |
| Ctrl-F / R | Find / Replace |
| Help | Help |
| Shift-Help | Context help (pointer becomes arrow + question mark) |
| Undo | Undo |

Text editing: Ctrl-B/I/U for bold/italic/underline; Ctrl-Backspace/Delete delete a word; Ctrl-Arrow moves a word; Ctrl-ClrHome/Shift-ClrHome go to document start/end; Shift-Arrows extend to line start/end and page up/down; Shift-Delete deletes a line.

Object editing: arrows select previous/next; Backspace/Delete deletes; Tab selects next; Shift-click extends the selection.

Notation: Control is `^`, Alternate is the window-closer glyph, Shift is the up arrow, function keys are `Fnn`, others are `Del`, `Bksp`, `Help`. Shortcuts are listed flush right in the menu. Do not duplicate a shortcut within a menu or dialog. Alt is discouraged as a modifier because international keyboards need it for characters.

**Web:** use the platform's conventions (`Cmd` on macOS, `Ctrl` elsewhere) and expose the same actions in menus. Show `^S`-style hints only in GEM-styled chrome; in body copy use the platform label.

## 7. Feedback and progress (777, TOS 10.2.5)

- For a long task, change the pointer to a busy bee; if the screen still updates, you may leave the arrow so the user can work elsewhere.
- A progress bar ("thermometer") reassures the user that the machine is not locked. Put it in a window so other applications remain reachable.
- State the remaining time when possible ("Time Left: xx:xx").
- The bar must move proportionally to real time - never 25% to 50% in ten seconds and 50% to 100% in one.
- Visual feedback is preferred to text, and must be immediate on selection and task start.

**Web:** real progress, not fake spinners. Indeterminate bars for unknown durations; `aria-valuenow` for known ones. Long jobs belong in a panel, not a blocking overlay.

## 8. Colour (787)

- Never alter the first 16 colours in 256-colour modes; change system colours only when absolutely necessary. The user owns them.
- To add a 3D effect under TOS 4, query `objc_sysvar()` and match the OS colours.
- Make dialogs `FL3DBAK` so the user's dialog colour shows through.
- Do not use colour to decorate; use it to emphasise or draw attention.
- Use colour for choices about colour, or where the user expects it in the document.
- When colour is a choice indicator, green is positive and red is negative.

**Web:** theme tokens, not hard-coded brand colours. One accent for meaning; everything else black/white/grey.

## 9. Sound (788)

Restrict sound to the system bell: a polite reminder when an operation is not possible (scrolling past the last line) or a signal that a long operation finished. Other sounds are allowed only if the user can enable them selectively.

**Web:** no sound by default. If you add it, make it opt-in, short, and never the only feedback.

## 10. Localisation (786, 789)

- Store every language-dependent string in the resource file; porting is then a resource edit.
- Allow at least 50% more space than the English text needs.
- Use the `_IDT` and `_AKP` cookies for dates, times, and currency.
- Never bake words into graphics.

**Web:** externalise strings, design for +50%, use `Intl` for dates/numbers/currency, and never put copy in images.

## 11. Device independence (785)

Use the VDI for all output; never write to hardware. Query the driver (`v_opnvwk`) instead of assuming a resolution, palette, or printer. Support GDOS for printing.

**Web:** feature-detect, don't sniff. Respect the user's font size, zoom, colour scheme, and input method.

## 12. Application, installation, entertainment (789-791)

- Ship `INSTALL.PRG`; let the user choose the install directory; back up any system file you modify and say where; tell the user when a reboot is needed; provide an uninstaller if removal is more than deleting a folder.
- Keep the program, resources, and readme in one directory; data in a child directory.
- Provide help in as many places as possible; context-sensitive help where possible.
- Never force a resolution change to run; return to the desktop in the resolution the user left; run in a window when possible.

**Web:** a real onboarding path, a settings page that remembers choices, and no forced full-screen. Provide a way back to the previous state.

## 13. The one-paragraph version

GEM's interface is a contract: the same menu positions, the same button order, the same meaning for grey and inverse, the same alert grammar. The look is plain because the behaviour is predictable. When building a GEM-styled page, spend the effort on the contract - titles that name the task, buttons that say what they do, errors that say what to do next - and let the pixels stay simple.
