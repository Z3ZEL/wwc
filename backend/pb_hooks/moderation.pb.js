/// <reference path="../pb_data/types.d.ts" />

// An admin editing someone else's campsite or comment may only change `hidden`.
// (The owner branch of the update rule already forbids owners from touching `hidden`.)
onRecordUpdateRequest((e) => {
  const isOwner = e.auth && e.record.getString("author") === e.auth.id;
  if (!e.hasSuperuserAuth() && !isOwner) {
    const body = e.requestInfo().body;
    const keys = Object.keys(body).filter((k) => k !== "hidden");
    if (keys.length > 0) {
      throw new ForbiddenError("Admins can only hide or unhide content.");
    }
  }
  e.next();
}, "campsites", "comments");
