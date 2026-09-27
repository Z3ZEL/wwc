//! Campsite panel: photos, details, rating, comments.

use std::borrow::Cow;

use egui::{Align, ImageSource, Layout, RichText, TextEdit, Ui};

use crate::actions::Action;
use crate::api::models::{Campsite, ReportTarget, date_part};
use crate::api::{PhotoSize, photo_url};
use crate::state::{AppState, CampsiteDetail, Panel, Remote, tent_capacity_label};
use crate::ui::theme::Theme;
use crate::ui::widgets::{
    SpinnerSize, StripStyle, Thumb, badge, danger_button, failed, form_error, link_button, loading, loading_block,
    muted, photo_strip, primary_button, section_title, spinner, stars, stars_input, tag_chip, verify_prompt,
};

pub fn show(ui: &mut Ui, state: &mut AppState, theme: &Theme, actions: &mut Vec<Action>) {
    let user_id = state.user_id().map(str::to_owned);
    // Email of a logged-in user who hasn't confirmed it yet (can't rate or comment).
    let unverified = state.session.as_ref().filter(|_| state.needs_verification()).map(|s| s.user.email.clone());
    if let Some(results) = state.search.results.loaded()
        && link_button(ui, theme, &format!("◀ Search results ({})", results.len())).clicked()
    {
        actions.push(Action::OpenPanel(Panel::Search));
    }
    let Some(d) = &mut state.detail else { return };

    let campsite = match &d.campsite {
        Remote::Loaded(c) => c.clone(),
        Remote::Failed(e) if e.is_not_found() => {
            muted(ui, theme, "This campsite doesn't exist anymore.");
            return;
        }
        Remote::Failed(e) => {
            if failed(ui, theme, e) {
                actions.push(Action::OpenPanel(Panel::Campsite(d.id.clone())));
            }
            return;
        }
        Remote::Loading | Remote::NotAsked => return loading_block(ui, theme, "Loading campsite…"),
    };
    let is_owner = user_id.as_deref() == Some(campsite.author.as_str());

    photos(ui, theme, &campsite, d, &state.api_base);
    header(ui, theme, &campsite, d, user_id.is_some(), is_owner, actions);
    ui.add_space(theme.spacing.section_gap);
    rating(ui, theme, d, user_id.is_some(), is_owner, unverified.as_deref(), actions);
    ui.add_space(theme.spacing.section_gap);
    comments(ui, theme, d, user_id.as_deref(), is_owner, unverified.as_deref(), actions);
}

fn header(
    ui: &mut Ui,
    theme: &Theme,
    c: &Campsite,
    d: &mut CampsiteDetail,
    logged_in: bool,
    is_owner: bool,
    actions: &mut Vec<Action>,
) {
    ui.label(RichText::new(&c.title).heading());
    let author = c.expand.author.as_ref().map_or("Unknown", |u| u.display_name());
    muted(ui, theme, format!("Added by {author} on {}", date_part(&c.created)));
    if c.hidden {
        badge(ui, theme, "Hidden by moderation", theme.colors.hidden_badge_bg, theme.colors.hidden_badge_text);
    }

    ui.horizontal(|ui| match &d.stats {
        Remote::Loaded(Some(s)) if s.rating_count > 0 => {
            stars(ui, theme, s.avg_score);
            ui.label(format!("{:.1}", s.avg_score));
            muted(ui, theme, format!("({} rating{})", s.rating_count, if s.rating_count == 1 { "" } else { "s" }));
        }
        Remote::Loaded(_) => {
            muted(ui, theme, "No ratings yet");
        }
        Remote::Loading | Remote::NotAsked => {
            spinner(ui, theme, SpinnerSize::Inline);
        }
        Remote::Failed(_) => {
            muted(ui, theme, "Ratings unavailable");
        }
    });

    ui.add_space(theme.spacing.item_spacing[1]);
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(format!("⛺ {} tents", tent_capacity_label(c.tent_capacity))).strong());
        for tag in &c.expand.tags {
            tag_chip(ui, theme, &tag.label, false);
        }
    });

    ui.horizontal(|ui| {
        let coords = format!("{:.5}, {:.5}", c.lat, c.lng);
        ui.monospace(&coords);
        if ui.small_button("Copy").on_hover_text("Copy coordinates").clicked() {
            ui.ctx().copy_text(coords);
            actions.push(Action::Toast("Coordinates copied.".into(), crate::actions::ToastKind::Info));
        }
    });

    if !c.description.is_empty() {
        ui.add_space(theme.spacing.item_spacing[1]);
        ui.label(&c.description);
    }

    if logged_in && !is_owner {
        ui.add_space(theme.spacing.item_spacing[1]);
        if link_button(ui, theme, "Report this campsite").clicked() {
            actions.push(Action::OpenPanel(Panel::Report(ReportTarget::Campsite(c.id.clone()))));
        }
    }

    if is_owner {
        ui.add_space(theme.spacing.item_spacing[1]);
        if d.confirm_delete {
            ui.label(RichText::new("Delete this campsite, with its ratings and comments?").color(theme.colors.danger));
            ui.horizontal(|ui| {
                if danger_button(ui, theme, if d.deleting { "Deleting…" } else { "Delete" }, !d.deleting).clicked() {
                    actions.push(Action::DeleteCampsite(c.id.clone()));
                }
                if ui.button("Keep it").clicked() {
                    d.confirm_delete = false;
                }
            });
        } else {
            ui.horizontal(|ui| {
                if ui.button("Edit").clicked() {
                    actions.push(Action::OpenPanel(Panel::EditCampsite(c.id.clone())));
                }
                if ui.button("Delete").clicked() {
                    d.confirm_delete = true;
                }
            });
        }
    }
}

/// Thumbnails; clicking one opens the full-page viewer (`ui::photo_viewer`).
fn photos(ui: &mut Ui, theme: &Theme, c: &Campsite, d: &mut CampsiteDetail, api_base: &str) {
    if c.photos.is_empty() {
        return;
    }
    let thumbs = c
        .photos
        .iter()
        .map(|file| Thumb {
            source: photo_url(api_base, c, file, PhotoSize::Thumb).map(|u| ImageSource::Uri(Cow::Owned(u))),
            label: file,
        })
        .collect();
    let style = StripStyle { height: theme.layout.photo_thumb_height, removable: false, selected: None };
    if let Some(i) = photo_strip(ui, theme, thumbs, style).clicked {
        d.photo_open = Some(i);
    }
    ui.add_space(theme.spacing.section_gap);
}

fn rating(
    ui: &mut Ui,
    theme: &Theme,
    d: &CampsiteDetail,
    logged_in: bool,
    is_owner: bool,
    unverified: Option<&str>,
    actions: &mut Vec<Action>,
) {
    if is_owner {
        return; // you can't rate your own campsite
    }
    section_title(ui, "Your rating");
    if !logged_in {
        login_prompt(ui, theme, "to rate this campsite.", actions);
        return;
    }
    if let Some(email) = unverified {
        verify_prompt(ui, theme, email, "to rate this campsite.", actions);
        return;
    }
    match &d.my_rating {
        Remote::Loaded(r) => {
            let current = r.as_ref().map(|r| r.score);
            if let Some(score) = stars_input(ui, theme, current, !d.rating_saving) {
                actions.push(Action::Rate(score));
            }
        }
        Remote::Failed(e) => {
            muted(ui, theme, format!("Rating unavailable: {e}"));
        }
        Remote::Loading | Remote::NotAsked => loading(ui, theme, "Loading your rating…"),
    }
}

fn comments(
    ui: &mut Ui,
    theme: &Theme,
    d: &mut CampsiteDetail,
    user_id: Option<&str>,
    is_owner: bool,
    unverified: Option<&str>,
    actions: &mut Vec<Action>,
) {
    ui.separator();
    section_title(ui, "Comments");

    match (user_id, unverified) {
        (Some(_), Some(email)) if !is_owner => verify_prompt(ui, theme, email, "to comment.", actions),
        (Some(_), None) if !is_owner => {
            ui.add(
                TextEdit::multiline(&mut d.comment_draft)
                    .hint_text("Share your experience…")
                    .desired_rows(3)
                    .char_limit(2000)
                    .desired_width(f32::INFINITY),
            );
            form_error(ui, theme, d.comment_error.as_ref());
            let can_post = !d.posting_comment && !d.comment_draft.trim().is_empty();
            if primary_button(ui, theme, if d.posting_comment { "Posting…" } else { "Post comment" }, can_post)
                .clicked()
            {
                actions.push(Action::PostComment);
            }
        }
        (Some(_), _) => {}
        (None, _) => login_prompt(ui, theme, "to comment.", actions),
    }
    ui.add_space(theme.spacing.item_spacing[1]);

    if d.comments.is_empty() && !d.comments_loading && d.comments_error.is_none() {
        muted(ui, theme, "No comments yet.");
    }
    for c in &d.comments {
        egui::Frame::NONE
            .fill(theme.colors.surface_alt)
            .corner_radius(theme.radius())
            .inner_margin(egui::Margin::same(10))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(RichText::new(c.author_name()).strong());
                    muted(ui, theme, date_part(&c.created));
                    if c.hidden {
                        badge(ui, theme, "Hidden", theme.colors.hidden_badge_bg, theme.colors.hidden_badge_text);
                    }
                    if user_id == Some(c.author.as_str()) {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.small_button("Delete").clicked() {
                                actions.push(Action::DeleteComment(c.id.clone()));
                            }
                        });
                    } else if user_id.is_some() {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.small_button("Report").on_hover_text("Report this comment").clicked() {
                                actions.push(Action::OpenPanel(Panel::Report(ReportTarget::Comment(c.id.clone()))));
                            }
                        });
                    }
                });
                ui.label(&c.body);
            });
    }

    if d.comments_loading && d.comments.is_empty() {
        loading_block(ui, theme, "Loading comments…");
    } else if d.comments_loading {
        loading(ui, theme, "Loading more comments…");
    } else if let Some(e) = &d.comments_error {
        muted(ui, theme, format!("Comments unavailable: {e}"));
    } else if d.has_more_comments() && ui.button("Load more").clicked() {
        actions.push(Action::LoadMoreComments);
    }
}

fn login_prompt(ui: &mut Ui, theme: &Theme, what: &str, actions: &mut Vec<Action>) {
    ui.horizontal(|ui| {
        if link_button(ui, theme, "Log in").clicked() {
            actions.push(Action::OpenPanel(Panel::Login));
        }
        ui.label(what);
    });
}
