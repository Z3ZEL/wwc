# 0020 — Forgot password: PocketBase's reset email and page

Status: proposed (2026-10-10).

## Context
A user who forgets their password has no way back into their account: the Profile panel can change the
password, but only with the old one. PocketBase has the whole flow built in: `request-password-reset`
emails a link with a one-time token, and its own page (`PB_APP_URL/_/#/auth/confirm-password-reset/…`)
sets the new password. SMTP is already configured for email confirmation (ADR 0012). Email confirmation
uses PocketBase's own page in the same way, and a custom page in the app is on the roadmap (§11).

## Decision
- **A "Forgot password?" link under the password field of the Login panel.** It opens a Reset password
  panel (`Panel::ResetPassword`, no route) with one email field, prefilled from the login form, and a
  "Send link" button (`Action::RequestPasswordReset` → `Event::PasswordResetRequested`).
- **The link in the email opens PocketBase's own page**, like email confirmation. That page sets the new
  password, and the user then comes back to log in. A page in the app would need a route for the token
  (hash routing is deferred, §5.3) and a second form to maintain. It can replace PocketBase's page
  together with the custom confirmation page.
- **The panel never says whether an account exists.** PocketBase answers 204 for every email and sends
  the mail in the background. After sending, the panel says "If an account uses <email>, we sent it a
  link", offers to send it again, and fills the login form's email. A rule test checks that a known and
  an unknown email get the same answer.
- **Reset emails get their own rate limit:** `*:requestPasswordReset`, 3 per 60 s per IP
  (`pb_settings.json`). The `*:auth` rule only covers the auth-with-* endpoints, so without it this
  endpoint would only fall under the general 300 per 10 s, and anyone could flood an inbox or the SMTP
  quota.

## Consequences
- Users can recover their account without contacting the operator.
- PocketBase's reset page looks like the dashboard, not like the app, and it is in English. Its link
  points to `PB_APP_URL`, which must be the public PocketBase URL (already required by ADR 0012).
- Setting the new password changes the account's token key, so every session of that account is
  logged out, including ones in other browsers.
- The email uses PocketBase's default "Reset password" template for the `users` collection. Changing its
  wording is done in the dashboard for now, or in a migration if it should live in the repo.
- Not done: a reset page inside the app, "Forgot your current password?" from the Profile panel, and a
  custom email template.
