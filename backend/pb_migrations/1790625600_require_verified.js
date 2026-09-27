/// <reference path="../pb_data/types.d.ts" />

// Creating content requires a confirmed email (ADR 0012, ARCHITECTURE §4.6).
// Updates and deletes keep their rules: an owner can always fix or remove their own content.

const VERIFIED = `@request.auth.verified = true && `;
const COLLECTIONS = ["campsites", "ratings", "comments", "reports"];

migrate((app) => {
  for (const name of COLLECTIONS) {
    const c = app.findCollectionByNameOrId(name);
    c.createRule = VERIFIED + c.createRule;
    app.save(c);
  }
}, (app) => {
  for (const name of COLLECTIONS) {
    const c = app.findCollectionByNameOrId(name);
    if (c.createRule.startsWith(VERIFIED)) {
      c.createRule = c.createRule.slice(VERIFIED.length);
      app.save(c);
    }
  }
});
