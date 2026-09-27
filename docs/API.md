# API — PocketBase collections

Source of truth: [`backend/pb_migrations/`](../backend/pb_migrations) (schema and rules) and [`backend/pb_hooks/`](../backend/pb_hooks). Permission tests: [`backend/tests/rules.sh`](../backend/tests/rules.sh).
The frontend calls these endpoints only from `frontend/src/api/client.rs`.

Shorthands: `AUTH` = `@request.auth.id != ""`, `ADMIN` = `@request.auth.role = "admin"`, `""` = public, `null` = superuser only.

## users (built-in auth collection)

| Field | Type | Notes |
|---|---|---|
| `name` | text | public display name |
| `avatar` | file | built-in, not used by the frontend yet |
| `role` | select `user` / `admin` | empty means `user`. Only a superuser can change it (dashboard). |

| Rule | Value |
|---|---|
| list | `id = @request.auth.id` (built-in: no user enumeration) |
| view | `""`: public profiles; emails stay hidden (`emailVisibility` = false) |
| create | `@request.body.role:isset = false` |
| update | `id = @request.auth.id && @request.body.role:isset = false` |
| delete | built-in: self only |

## tags

`slug` (unique, `^[a-z0-9_]+$`), `label`, `icon`, `sort_order`, `active`. Seeded: `safe_water`, `river`, `flat`.
Rules: list/view `""`; create/update/delete `null` (managed in the dashboard). The frontend lists `active = true` tags sorted by `sort_order`.

## campsites

| Field | Type | Notes |
|---|---|---|
| `title` | text | required, 3–100 |
| `description` | text | ≤ 5000, plain text |
| `lat`, `lng` | number | −90..90, −180..180. Not `required`, because PocketBase treats 0 as empty. |
| `tags` | relation → tags (multi) | |
| `tent_capacity` | number | integer 1–10, where 10 is shown as "10+" |
| `photos` | file (multi) | at most 3; `image/jpeg`, `image/png`, `image/webp`; 10 MB each; thumbs `320x240`, `1200x1200f`; not protected |
| `hidden` | bool | moderation flag, admins only |
| `author` | relation → users | cascade delete |
| `created`, `updated` | autodate | |

| Rule | Value |
|---|---|
| list / view | `hidden = false \|\| author = @request.auth.id \|\| ADMIN` |
| create | `AUTH && @request.body.author = @request.auth.id && @request.body.hidden:isset = false` |
| update | `(author = @request.auth.id && @request.body.author:isset = false && @request.body.hidden:isset = false) \|\| ADMIN` (hook: admins may only change `hidden`) |
| delete | `author = @request.auth.id \|\| ADMIN` |

Indexes: `(lat, lng)`, `(author)`.

**Photos** follow the campsite rules: only the owner can add or remove them, and the moderation hook rejects
`photos+`/`photos-` from admins (the same as any non-`hidden` change). When photos change, the frontend sends
one `multipart/form-data` request: a `@jsonPayload` part holds the usual JSON body (with `"photos-": [filenames]`
to delete), plus one `photos+` part per new file. A save without new files stays plain JSON (`photos-` works there too).
Files are public: `GET /api/files/{collectionId}/{campsiteId}/{filename}?thumb=320x240|1200x1200f`. The frontend
never loads originals. See ADR 0010.

## ratings

`campsite` (cascade), `author` (cascade), `score` (integer 1–5). Unique index `(campsite, author)`: one rating per user per campsite. To re-rate, update the existing record.

| Rule | Value |
|---|---|
| list / view | `""` |
| create | `AUTH && @request.body.author = @request.auth.id && @request.body.campsite.author != @request.auth.id` |
| update | `author = @request.auth.id && @request.body.author:isset = false && @request.body.campsite:isset = false` |
| delete | `author = @request.auth.id` |

## comments

`campsite` (cascade), `author` (cascade), `body` (1–2000), `hidden`.

| Rule | Value |
|---|---|
| list / view | `hidden = false \|\| author = @request.auth.id \|\| ADMIN` |
| create | `AUTH && @request.body.author = @request.auth.id && @request.body.campsite.author != @request.auth.id && @request.body.hidden:isset = false` |
| update | `(author = @request.auth.id && …author/campsite/hidden not set) \|\| ADMIN` (hook: admins may only change `hidden`) |
| delete | `author = @request.auth.id \|\| ADMIN` |

## reports

A logged-in user flags a campsite or a comment for admin review. Admins read and resolve them in the dashboard (`/_/`); the app has no report list yet.

| Field | Type | Notes |
|---|---|---|
| `reporter` | relation → users | required, = auth user, cascade delete |
| `campsite` | relation → campsites | cascade delete. Exactly one of `campsite` / `comment` is set (hook). |
| `comment` | relation → comments | cascade delete |
| `reason` | select | `spam`, `inappropriate`, `wrong_location` (campsites only, hook), `dangerous`, `other` |
| `details` | text | ≤ 1000. The frontend requires it when the reason is `other`. |
| `status` | select | `open` (set by the hook on create), `resolved`, `dismissed` |
| `created`, `updated` | autodate | |

| Rule | Value |
|---|---|
| list / view | `reporter = @request.auth.id \|\| ADMIN` |
| create | `AUTH && @request.body.reporter = @request.auth.id && @request.body.status:isset = false` |
| update | `ADMIN` |
| delete | `ADMIN` |

Indexes: `(status, created)`; partial unique `(reporter, campsite) WHERE campsite != ''` and `(reporter, comment) WHERE comment != ''` (one report per user per target; partial so the unset target `''` never collides).

## campsite_stats (view, read-only)

One row per visible campsite, with the same `id` as the campsite: `avg_score` (0 when there are no ratings), `rating_count`, `comment_count` (visible comments only). Rules: list/view `""`.

## Hooks

| File | What |
|---|---|
| `own_campsite.pb.js` | Rejects a rating or comment on your own campsite (backup for the create rules). |
| `reports.pb.js` | On report create: exactly one target, no `wrong_location` on a comment, not your own content, no duplicate ("You already reported this."), and `status = open`. |
| `moderation.pb.js` | When an admin who isn't the author updates a campsite or comment, only `hidden` may change (this also blocks photo uploads and removals). |

## Requests the frontend makes

```
GET    /api/collections/campsites/records?perPage=1&fields=id                  → totalItems (top bar count)
GET    /api/collections/campsites/records?filter=lat >= S && lat <= N && lng >= W && lng <= E
                                         &fields=id,title,lat,lng,author&perPage=200&skipTotal=true
       … + active filters, each clause parenthesized and AND-combined:
         (tags ~ 'TAG_ID') per selected tag, (tent_capacity >= MIN) if MIN > 1, (tent_capacity <= MAX) if MAX < 10
GET    /api/collections/campsites/records?filter=(title ~ 'QUERY') && <filters above>
                                         &fields=id,title,lat,lng,author&sort=title&perPage=50   (top bar search)
GET    /api/collections/campsites/records/:id?expand=author,tags
GET    /api/collections/campsite_stats/records/:id                               (404 → no stats)
GET    /api/collections/comments/records?filter=campsite='ID'&sort=-created&expand=author&page=N&perPage=20
GET    /api/collections/ratings/records?filter=campsite='ID' && author='UID'&perPage=1
GET    /api/collections/tags/records?filter=active = true&sort=sort_order,label&perPage=200
POST   /api/collections/users/auth-with-password   {identity, password}
POST   /api/collections/users/auth-refresh
POST   /api/collections/users/records               {name, email, password, passwordConfirm}
PATCH  /api/collections/users/records/:id           {name} | {oldPassword, password, passwordConfirm}
POST   /api/collections/campsites/records?expand=author,tags   {title, description, lat, lng, tags, tent_capacity, author}
PATCH  /api/collections/campsites/records/:id?expand=author,tags   (same, without author; + "photos-": [filenames])
       … either one as multipart/form-data when there are new photos: @jsonPayload=<JSON above>, photos+=<file> (×n)
GET    /api/files/:collectionId/:id/:filename?thumb=320x240 | 1200x1200f            (photos, via egui's image loader)
DELETE /api/collections/campsites/records/:id
POST   /api/collections/ratings/records             {campsite, author, score}
PATCH  /api/collections/ratings/records/:id         {score}
POST   /api/collections/comments/records?expand=author   {campsite, author, body}
DELETE /api/collections/comments/records/:id
POST   /api/collections/reports/records                  {reporter, campsite | comment, reason, details}
```

Ids are checked (`is_record_id`: alphanumeric only) before they go into a URL path or filter. Search text goes
through `clean_search` (drops `'` `"` `\` backtick and control characters, max 100 chars) before it is quoted.
Tag filters use `tags ~ 'ID'`: with PocketBase 0.40.4, `tags ?= 'ID'` never matches (ADR 0011). Photo URLs
(`photo_url`) also check the collection id (`[A-Za-z0-9_]`) and the filename (`[A-Za-z0-9_.-]`, no leading dot).
