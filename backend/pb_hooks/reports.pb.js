/// <reference path="../pb_data/types.d.ts" />

// Reports: exactly one target (campsite or comment), never your own content, one report per
// user per target, and every new report starts `open`. The unique indexes are the safety net.
onRecordCreateRequest((e) => {
  const campsiteId = e.record.getString("campsite");
  const commentId = e.record.getString("comment");
  if (!campsiteId === !commentId) {
    throw new BadRequestError("Report either a campsite or a comment.");
  }
  if (commentId && e.record.getString("reason") === "wrong_location") {
    throw new BadRequestError("Only a campsite can have a wrong location.");
  }

  if (!e.hasSuperuserAuth()) {
    const [collection, id, field] = campsiteId
      ? ["campsites", campsiteId, "campsite"]
      : ["comments", commentId, "comment"];
    let target;
    try {
      target = e.app.findRecordById(collection, id);
    } catch (_) {
      throw new BadRequestError("This content doesn't exist anymore.");
    }
    if (e.auth && target.getString("author") === e.auth.id) {
      throw new BadRequestError("You can't report your own content.");
    }
    let duplicate = null;
    try {
      duplicate = e.app.findFirstRecordByFilter("reports", `reporter = {:reporter} && ${field} = {:id}`, {
        reporter: e.record.getString("reporter"),
        id,
      });
    } catch (_) {
      // not found: first report on this target
    }
    if (duplicate) {
      throw new BadRequestError("You already reported this.");
    }
  }

  e.record.set("status", "open");
  e.next();
}, "reports");
