/// <reference path="../pb_data/types.d.ts" />

// Campsite photos: up to 3 images, uploaded by the owner. See docs/adr/0010-campsite-photos.md.
// Uploads go through the existing campsites create/update rules (owner only); the admin hook
// (pb_hooks/moderation.pb.js) keeps admins from touching them.
// Keep the limits in sync with frontend/src/state/forms.rs (MAX_PHOTOS, MAX_PHOTO_BYTES, PHOTO_MIME_TYPES).

migrate((app) => {
  const campsites = app.findCollectionByNameOrId("campsites");
  campsites.fields.add(new FileField({
    name: "photos",
    maxSelect: 3,
    maxSize: 10 * 1024 * 1024,
    mimeTypes: ["image/jpeg", "image/png", "image/webp"],
    // 320x240: panel thumbnails (center crop). 1200x1200f: the full-page photo viewer, fitted inside
    // 1200x1200 so it always stays far below the GPU texture size limit.
    thumbs: ["320x240", "1200x1200f"],
    protected: false,
  }));
  app.save(campsites);
}, (app) => {
  const campsites = app.findCollectionByNameOrId("campsites");
  campsites.fields.removeByName("photos");
  app.save(campsites);
});
