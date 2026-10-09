# 0018 — Password fields on phones: two fixes to eframe's text input

Status: accepted (2026-10-09).

## Context
On phones, typing in a password field (log in, sign up, change password) filled nothing, and odd glyphs
showed next to the cursor. Desktop browsers were fine.

egui draws every field on the canvas. To get keyboard input, eframe's web backend (0.36.2) focuses a hidden
1 px `<input type="text">` it places right after the canvas: the "text agent". While a password field has
focus, eframe only accepts `insertText` input events from it, then clears it, so the password never stays
in the DOM. Phone keyboards such as Gboard treat a text input as normal text: they compose each word
(`insertCompositionText` events), so eframe dropped every key. The text piled up in the hidden input, the
suggestion bar showed it in clear, and Backspace was lost too (key code 229 + `deleteContentBackward`).
The hidden input also had a visible text color, so what piled up in it showed by the cursor. That part was
fixed upstream in egui PR #8550, but after 0.36.2, the latest release. The composition part is not fixed
upstream.

ARCHITECTURE §5.1 says not to work around canvas text input with DOM changes unless an ADR says so.

## Decision
- **Hide the text agent** with a CSS rule in `index.html`
  (`canvas#app + input { color: transparent; opacity: 0; pointer-events: none; }`): the #8550 fix.
- **Password input while a password field is focused, on touch screens.** At the end of each frame,
  `app.rs` checks whether egui reports a password field (`PlatformOutput::ime` with
  `IMEPurpose::Password`) and whether a touch screen was ever used (`InputState::has_touch_screen`). When
  that changes, `web::set_password_mode` sets the text agent's `type` to `password` or back to `text`. A
  phone keyboard then switches to its password mode: no composition and no suggestions, so each key arrives
  as `insertText`, the one path eframe handles. The hidden input is still emptied after each key, so
  Backspace reaches egui as a normal key.
- Desktop is left as it was: eframe keeps the input `type="text"` on purpose, because a desktop password
  manager would otherwise take the last typed letter for the password.

## Consequences
- Password fields work on phones, and the keyboard no longer shows or learns typed passwords.
- A laptop with a touch screen gets the password input once it has been touched; typing still works there.
- Both fixes depend on how eframe builds its text agent (its place after the canvas, its event handling).
  Check them on each egui/eframe upgrade: drop the CSS rule once the upgrade contains #8550, and the type
  switch once eframe handles password input on phones itself.
- Not done: forking eframe (a patch to maintain for one input); waiting for a release (nothing upstream
  fixes the composition part); a normal field drawn with dots (the keyboard would see the password as text
  and suggest it).
