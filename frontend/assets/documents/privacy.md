<!--
  Privacy Policy (GDPR articles 13 and 14). Shown in the app by the Document panel (ARCHITECTURE §5.11).
  Values in double braces come from documents.json (vars). Bump "updated" there when you change this file:
  the app then shows its privacy notice again.
  Keep it true to the code: browser storage keys (frontend/src/app.rs), log retention
  (backend/pb_settings.json), backups (PB_BACKUPS_MAX_KEEP), public API rules (docs/API.md).
-->

> **In short:** we only collect what the site needs to work: your email address and display name, what you post, and short-lived technical logs. No ads, no analytics, no tracking, and we never sell data. What you post is public. You can ask for a copy of your data, or for its deletion, at any time.

## 1. Who is responsible

The controller of your personal data is {{operator_name}}, {{operator_address}}, [{{contact_email}}](mailto:{{contact_email}}). Write to this address for any question about your data, or to use your rights.

## 2. What we collect, why, and for how long

### When you visit the site, with or without an account

**Technical logs.** When your browser loads the site or calls our server, the request is logged: IP address, date and time, the address requested, and your browser's type. We use these logs to keep the service secure and working, for example to limit abusive traffic, and to fix errors.

_Legal basis:_ our legitimate interest in running a secure service (GDPR, article 6(1)(f)). _Kept:_ {{log_retention_days}} days on our server. Our hosting providers keep their own access logs, as described in their policies.

**Map images.** The map background is loaded directly from the servers of the OpenStreetMap Foundation, which therefore receive your IP address and the address of the page, like any website you visit. The Foundation is an independent controller: see its [privacy policy](https://osmfoundation.org/wiki/Privacy_Policy).

_Legal basis:_ our legitimate interest in showing a map (article 6(1)(f)).

**Browser storage.** See section 5.

### Your account

- **Email address:** to log you in, confirm your address, and contact you about your account or content. It is never shown publicly.
- **Password:** stored only as a secure hash. We can't read it.
- **Display name:** shown with everything you post. Your public profile also shows technical details, such as when the account was created and whether its email is confirmed.
- **Account status:** whether your email is confirmed, and your role (user or administrator).

An email address and a password are needed to create an account; without an account you can still browse everything.

_Legal basis:_ the performance of our [Terms of Use](terms.md), which you accept when you sign up (article 6(1)(b)). _Kept:_ until you delete your account.

### What you post

- **Campsites:** title, description, exact location, tags, tent capacity, photos, and when they were added or changed.
- **Ratings and comments,** with the campsite they belong to.

All of this is **public**, with your display name. The site shows the average of the ratings, but individual ratings, and the account that gave them, can also be read by anyone through our public interface.

**Photos are published as you upload them.** Photo files often contain hidden metadata, such as the GPS position where the photo was taken, the date and the camera model. Anyone can download the original file, so remove this metadata before you upload a photo if you don't want to share it.


_Legal basis:_ the performance of the Terms of Use, since publishing is the service you ask for (article 6(1)(b)). _Kept:_ until you delete the item or your account, or until moderation removes it.

### Reports and messages

When you report content, we store your account, the content, the reason you chose and your explanation. Only administrators see reports: the author of the reported content doesn't see who reported it. When you write to us, for example to report illegal content without an account, we keep your email address and your message.

_Legal basis:_ our legal obligations under the EU Digital Services Act (article 6(1)(c)) and our legitimate interest in keeping the site safe and lawful (article 6(1)(f)). _Kept:_ reports until they or the account that sent them are deleted; messages for as long as we need them to handle the matter.

### Emails we send

Only emails about your account: the address confirmation, and messages about your account or content (for example a moderation decision). No newsletter, no marketing. They go through our email delivery provider.

### What we don't do

No advertising, no analytics or audience measurement, no profiling, no automated decisions about you, no sale or rental of data.

## 3. Who receives your data

The site runs on service providers that process data on our behalf and on our instructions (processors):

- website hosting: {{frontend_host}};
- server and database hosting: {{backend_host}};
- storage of photos and backups: {{storage_provider}};
- email delivery: {{email_provider}}.

The OpenStreetMap Foundation receives the requests for map images as an independent controller (section 2). We may also disclose data when the law requires it, for example to a court or an authority.

## 4. Transfers outside the European Union

Some of these providers may process data outside the European Economic Area: our website host, for example, is based in the United States. Such transfers rely on an adequacy decision of the European Commission (such as the EU–US Data Privacy Framework, for certified companies) or on the Commission's standard contractual clauses. Write to us for more information or a copy of these safeguards.

## 5. Browser storage and cookies

The site doesn't use cookies. It keeps two items in your browser's local storage, only because you need them:

- `wwc_session`: your login session (a token and your basic account details), so you stay logged in. It is cleared when you log out, and stops working when the session expires.
- `wwc_notice`: that you closed the privacy notice, so it doesn't come back until this policy changes.

Both are strictly necessary for the service you ask for, so they don't need your consent. There are no analytics, advertising or social media trackers. You can delete them at any time in your browser settings.

<!--
  Draft for when consent is enabled (frontend/assets/consent.json, ARCHITECTURE §5.12). Replace the
  paragraph above with it, fill in the analytics details, and bump "updated" in documents.json.

- `wwc_consent`: the choices you made in the "Cookies and analytics" panel, and when, so we don't ask
  again for 6 months. It is needed to respect your choice, so it doesn't need consent either.

**Audience measurement, only if you accept it.** With your consent, we measure how the site is used:
[which data: pages or panels opened, approximate location from the IP address, browser type…], with
[analytics provider, and where it processes data]. [What it stores in your browser, and for how long.]
We use it only to improve the site, never for advertising.
_Legal basis:_ your consent (article 6(1)(a)). You can withdraw it at any time with "Privacy choices" at
the bottom of the map; this doesn't affect what was measured before. _Kept:_ [retention].

There are no advertising or social media trackers.
-->

## 6. How long we keep data

- **Account:** until you delete it.
- **Content you posted:** until you delete it, moderation removes it, or you delete your account.
- **Server logs:** {{log_retention_days}} days.
- **Backups:** the server is backed up every day, and each backup is kept {{backup_retention_days}} days. Deleted data therefore disappears from backups within {{backup_retention_days}} days.
- **Messages you send us:** as long as needed to handle them.

## 7. Your rights

Under the GDPR, you have the right to:

- **access** your data and get a copy of it;
- **correct** it: you can change your display name in your Profile and edit your campsites yourself;
- **erase** it: deleting your account also deletes your campsites, photos, ratings, comments and reports;
- **receive** the data you gave us in a machine-readable format (**portability**);
- **object** to processing based on our legitimate interest, and ask us to **restrict** processing.

For now, account deletion and data export are done by email: write to [{{contact_email}}](mailto:{{contact_email}}) from the address of your account. We answer within one month, and may ask you to confirm that the account is yours.

You can also lodge a complaint with a data protection authority, in particular in the EU country where you live or work, or where you think a breach took place. Ours is: {{supervisory_authority}}.

## 8. Security

The site is served over HTTPS only. Passwords are hashed, access to data is controlled by rules enforced on the server, and administrator access is restricted.

## 9. Children

The site is not intended for children: you must be at least {{min_age}} years old to create an account. If you think a child has given us personal data, contact us and we will delete it.

## 10. Changes to this policy

We update this policy when our practices or the law change. The date at the top shows the current version, and the site shows a notice when it changes.
