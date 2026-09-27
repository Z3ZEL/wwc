/// <reference path="../pb_data/types.d.ts" />

// Users can't rate or comment on their own campsite.
// Also enforced by the collection create rules; this is defense in depth.
onRecordCreateRequest((e) => {
  const campsiteId = e.record.getString("campsite");
  if (e.auth && campsiteId) {
    const campsite = e.app.findRecordById("campsites", campsiteId);
    if (campsite.getString("author") === e.auth.id) {
      throw new BadRequestError("You can't rate or comment on your own campsite.");
    }
  }
  e.next();
}, "ratings", "comments");
