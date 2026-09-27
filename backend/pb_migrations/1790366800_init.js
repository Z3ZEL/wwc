/// <reference path="../pb_data/types.d.ts" />

// Initial schema. See docs/ARCHITECTURE.md §4.2–4.3 and docs/API.md.
// Coordinates are two number fields (lat/lng) so bbox filters and indexes stay simple.

const ADMIN = `@request.auth.role = "admin"`;
const AUTH = `@request.auth.id != ""`;

migrate((app) => {
  // ---- users: add role, public profile view, users can't set their own role
  const users = app.findCollectionByNameOrId("users");
  users.fields.add(new SelectField({
    name: "role",
    values: ["user", "admin"],
    maxSelect: 1,
  }));
  users.viewRule = ""; // public profiles (emails stay hidden via emailVisibility)
  users.createRule = `@request.body.role:isset = false`;
  users.updateRule = `id = @request.auth.id && @request.body.role:isset = false`;
  app.save(users);

  // ---- tags (admin-managed lookup table)
  const tags = new Collection({
    type: "base",
    name: "tags",
    listRule: "",
    viewRule: "",
    createRule: null,
    updateRule: null,
    deleteRule: null,
    fields: [
      { name: "slug", type: "text", required: true, max: 50, pattern: "^[a-z0-9_]+$" },
      { name: "label", type: "text", required: true, max: 50 },
      { name: "icon", type: "text", max: 50 },
      { name: "sort_order", type: "number", onlyInt: true },
      { name: "active", type: "bool" },
    ],
    indexes: ["CREATE UNIQUE INDEX idx_tags_slug ON tags (slug)"],
  });
  app.save(tags);

  [
    ["safe_water", "Safe water", 1],
    ["river", "River", 2],
    ["flat", "Flat ground", 3],
  ].forEach(([slug, label, order]) => {
    const r = new Record(tags);
    r.set("slug", slug);
    r.set("label", label);
    r.set("sort_order", order);
    r.set("active", true);
    app.save(r);
  });

  // ---- campsites
  const campsites = new Collection({
    type: "base",
    name: "campsites",
    listRule: `hidden = false || author = @request.auth.id || ${ADMIN}`,
    viewRule: `hidden = false || author = @request.auth.id || ${ADMIN}`,
    createRule: `${AUTH} && @request.body.author = @request.auth.id && @request.body.hidden:isset = false`,
    // Admins may only toggle `hidden` (enforced in pb_hooks/moderation.pb.js).
    updateRule: `(author = @request.auth.id && @request.body.author:isset = false && @request.body.hidden:isset = false) || ${ADMIN}`,
    deleteRule: `author = @request.auth.id || ${ADMIN}`,
    fields: [
      { name: "title", type: "text", required: true, min: 3, max: 100 },
      { name: "description", type: "text", max: 5000 },
      // Not `required`: PocketBase treats 0 as empty, and 0 is a valid coordinate.
      { name: "lat", type: "number", min: -90, max: 90 },
      { name: "lng", type: "number", min: -180, max: 180 },
      { name: "tags", type: "relation", collectionId: tags.id, maxSelect: 20 },
      { name: "tent_capacity", type: "number", required: true, onlyInt: true, min: 1, max: 10 },
      { name: "hidden", type: "bool" },
      { name: "author", type: "relation", collectionId: users.id, maxSelect: 1, required: true, cascadeDelete: true },
      { name: "created", type: "autodate", onCreate: true, onUpdate: false },
      { name: "updated", type: "autodate", onCreate: true, onUpdate: true },
    ],
    indexes: [
      "CREATE INDEX idx_campsites_lat_lng ON campsites (lat, lng)",
      "CREATE INDEX idx_campsites_author ON campsites (author)",
    ],
  });
  app.save(campsites);

  // ---- ratings (one per user per campsite, never on your own campsite)
  const ratings = new Collection({
    type: "base",
    name: "ratings",
    listRule: "",
    viewRule: "",
    createRule: `${AUTH} && @request.body.author = @request.auth.id && @request.body.campsite.author != @request.auth.id`,
    updateRule: `author = @request.auth.id && @request.body.author:isset = false && @request.body.campsite:isset = false`,
    deleteRule: `author = @request.auth.id`,
    fields: [
      { name: "campsite", type: "relation", collectionId: campsites.id, maxSelect: 1, required: true, cascadeDelete: true },
      { name: "author", type: "relation", collectionId: users.id, maxSelect: 1, required: true, cascadeDelete: true },
      { name: "score", type: "number", required: true, onlyInt: true, min: 1, max: 5 },
      { name: "created", type: "autodate", onCreate: true, onUpdate: false },
      { name: "updated", type: "autodate", onCreate: true, onUpdate: true },
    ],
    indexes: ["CREATE UNIQUE INDEX idx_ratings_campsite_author ON ratings (campsite, author)"],
  });
  app.save(ratings);

  // ---- comments
  const comments = new Collection({
    type: "base",
    name: "comments",
    listRule: `hidden = false || author = @request.auth.id || ${ADMIN}`,
    viewRule: `hidden = false || author = @request.auth.id || ${ADMIN}`,
    createRule: `${AUTH} && @request.body.author = @request.auth.id && @request.body.campsite.author != @request.auth.id && @request.body.hidden:isset = false`,
    updateRule: `(author = @request.auth.id && @request.body.author:isset = false && @request.body.campsite:isset = false && @request.body.hidden:isset = false) || ${ADMIN}`,
    deleteRule: `author = @request.auth.id || ${ADMIN}`,
    fields: [
      { name: "campsite", type: "relation", collectionId: campsites.id, maxSelect: 1, required: true, cascadeDelete: true },
      { name: "author", type: "relation", collectionId: users.id, maxSelect: 1, required: true, cascadeDelete: true },
      { name: "body", type: "text", required: true, min: 1, max: 2000 },
      { name: "hidden", type: "bool" },
      { name: "created", type: "autodate", onCreate: true, onUpdate: false },
      { name: "updated", type: "autodate", onCreate: true, onUpdate: true },
    ],
    indexes: ["CREATE INDEX idx_comments_campsite ON comments (campsite, created)"],
  });
  app.save(comments);

  // ---- campsite_stats (read-only view, one row per visible campsite)
  const stats = new Collection({
    type: "view",
    name: "campsite_stats",
    listRule: "",
    viewRule: "",
    viewQuery: `
      SELECT
        c.id AS id,
        CAST(COALESCE((SELECT AVG(r.score) FROM ratings r WHERE r.campsite = c.id), 0) AS REAL) AS avg_score,
        CAST((SELECT COUNT(*) FROM ratings r WHERE r.campsite = c.id) AS INTEGER) AS rating_count,
        CAST((SELECT COUNT(*) FROM comments m WHERE m.campsite = c.id AND m.hidden = FALSE) AS INTEGER) AS comment_count
      FROM campsites c
      WHERE c.hidden = FALSE
    `,
  });
  app.save(stats);
}, (app) => {
  for (const name of ["campsite_stats", "comments", "ratings", "campsites", "tags"]) {
    app.delete(app.findCollectionByNameOrId(name));
  }
  const users = app.findCollectionByNameOrId("users");
  users.fields.removeByName("role");
  app.save(users);
});
