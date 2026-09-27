/// <reference path="../pb_data/types.d.ts" />

// Reports: a logged-in user flags a campsite or a comment for admin review (ADR 0006, ARCHITECTURE §5.8).
// Admins read them in the dashboard for now. pb_hooks/reports.pb.js checks the target and sets `status`.
// Keep the reasons in sync with `ReportReason` in frontend/src/api/models.rs.

const ADMIN = `@request.auth.role = "admin"`;
const AUTH = `@request.auth.id != ""`;

migrate((app) => {
  const users = app.findCollectionByNameOrId("users");
  const campsites = app.findCollectionByNameOrId("campsites");
  const comments = app.findCollectionByNameOrId("comments");

  const reports = new Collection({
    type: "base",
    name: "reports",
    listRule: `reporter = @request.auth.id || ${ADMIN}`,
    viewRule: `reporter = @request.auth.id || ${ADMIN}`,
    createRule: `${AUTH} && @request.body.reporter = @request.auth.id && @request.body.status:isset = false`,
    updateRule: ADMIN,
    deleteRule: ADMIN,
    fields: [
      { name: "reporter", type: "relation", collectionId: users.id, maxSelect: 1, required: true, cascadeDelete: true },
      // Exactly one target is set (enforced by the hook).
      { name: "campsite", type: "relation", collectionId: campsites.id, maxSelect: 1, cascadeDelete: true },
      { name: "comment", type: "relation", collectionId: comments.id, maxSelect: 1, cascadeDelete: true },
      {
        name: "reason",
        type: "select",
        required: true,
        maxSelect: 1,
        values: ["spam", "inappropriate", "wrong_location", "dangerous", "other"],
      },
      { name: "details", type: "text", max: 1000 },
      { name: "status", type: "select", maxSelect: 1, values: ["open", "resolved", "dismissed"] },
      { name: "created", type: "autodate", onCreate: true, onUpdate: false },
      { name: "updated", type: "autodate", onCreate: true, onUpdate: true },
    ],
    indexes: [
      "CREATE INDEX idx_reports_status ON reports (status, created)",
      // One report per user per target. Partial: the unset target is '' and must not collide.
      "CREATE UNIQUE INDEX idx_reports_reporter_campsite ON reports (reporter, campsite) WHERE campsite != ''",
      "CREATE UNIQUE INDEX idx_reports_reporter_comment ON reports (reporter, comment) WHERE comment != ''",
    ],
  });
  app.save(reports);
}, (app) => {
  app.delete(app.findCollectionByNameOrId("reports"));
});
