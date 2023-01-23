use std::{collections::VecDeque, time::Duration};

use chrono::NaiveDate;
use poise::{
    serenity_prelude::{ButtonStyle, CreateComponents, CreateEmbed, EditMessage, User},
    CreateReply,
};
use songbird::{input::Metadata, tracks::TrackHandle};

use crate::types::PAGE_SIZE;

pub(crate) fn format_duration(duration: &Duration) -> String {
    let secs = duration.as_secs();
    let mins = secs / 60;
    let secs = secs % 60;
    format!("{mins}:{secs:02}")
}

pub(crate) fn base_embed(e: &mut CreateEmbed) -> &mut CreateEmbed {
    e.color(0x0789f0)
}

pub(crate) fn song_embed<'e>(mut e: &'e mut CreateEmbed, song: &Metadata) -> &'e mut CreateEmbed {
    e = base_embed(e);

    if let Some(title) = &song.title {
        e = e.title(title);
    }

    if let Some(url) = &song.source_url {
        e = e.url(url);
    }

    if let Some(artist) = &song.artist {
        e = e.author(|a| a.name(artist));
    } else if let Some(channel) = &song.channel {
        e = e.author(|a| a.name(channel));
    }

    if let Some(url) = &song.thumbnail {
        e = e.image(url);
    }

    let mut footer = vec![];

    if let Some(duration) = &song.duration {
        footer.push(format_duration(duration));
    }

    if let Some(date) = song.date.as_ref().and_then(|d| {
        let Some(year) = d.get(0..4).and_then(|s| s.parse().ok()) else { return None; };
        let Some(month) = d.get(4..6).and_then(|s| s.parse().ok()) else { return None; };
        let Some(day) = d.get(6..8).and_then(|s| s.parse().ok()) else { return None; };
        let Some(date) = NaiveDate::from_ymd_opt(year, month, day) else { return None; };
        Some(date.format("Uploaded on %Y/%m/%d"))
    }) {
        footer.push(date.to_string());
    }

    if !footer.is_empty() {
        e = e.footer(|f| f.text(footer.join(" • ")));
    }

    e
}

pub(crate) fn now_playing_message<'m, 'att>(
    mut m: &'m mut CreateReply<'att>,
    song: &Metadata,
) -> &'m mut CreateReply<'att> {
    if let Some(title) = &song.title {
        m = m.content(format!("Now playing *{title}*."));
    } else {
        m = m.content("Now playing a new song.");
    }

    m.embed(|e| song_embed(e, song))
}

fn create_queue_embed<'e>(
    mut e: &'e mut CreateEmbed,
    np: &TrackHandle,
    queue: &VecDeque<(usize, &TrackHandle)>,
    page: usize,
    total_pages: usize,
) -> &'e mut CreateEmbed {
    e = base_embed(e).title("Queue").field(
        "Now Playing",
        format!(
            "[{}]({}) `{}`",
            np.metadata().title.as_ref().unwrap(),
            np.metadata().source_url.as_ref().unwrap(),
            format_duration(np.metadata().duration.as_ref().unwrap()),
        ),
        false,
    );

    if !queue.is_empty() {
        e = e
            .field(
                format!("Page {}", page + 1),
                queue
                    .iter()
                    .map(|(i, song)| {
                        let metadata = song.metadata();
                        format!(
                            "*{i}.* [{title}]({url}) `{duration}`",
                            title = metadata.title.as_ref().unwrap(),
                            url = metadata.source_url.as_ref().unwrap(),
                            duration = format_duration(metadata.duration.as_ref().unwrap())
                        )
                    })
                    .skip(page * PAGE_SIZE)
                    .take(PAGE_SIZE)
                    .collect::<Vec<_>>()
                    .join("\n"),
                false,
            )
            .footer(|f| f.text(format!("{}/{}", page + 1, total_pages)));
    }

    e
}

fn create_queue_components(
    c: &mut CreateComponents,
    page: usize,
    total_pages: usize,
    disabled: bool,
) -> &mut CreateComponents {
    c.create_action_row(|r| {
        r.create_button(|b| {
            b.custom_id("first")
                .label("◀◀")
                .style(ButtonStyle::Primary)
                .disabled(disabled || page == 0)
        })
        .create_button(|b| {
            b.custom_id("previous")
                .label("◀")
                .style(ButtonStyle::Primary)
                .disabled(disabled || page == 0)
        })
        .create_button(|b| {
            b.custom_id("next")
                .label("▶")
                .style(ButtonStyle::Primary)
                .disabled(disabled || page >= total_pages - 1)
        })
        .create_button(|b| {
            b.custom_id("last")
                .label("▶▶")
                .style(ButtonStyle::Primary)
                .disabled(disabled || page >= total_pages - 1)
        })
    })
}

pub(crate) fn queue_message<'m, 'att>(
    m: &'m mut CreateReply<'att>,
    queue: &[TrackHandle],
    page: usize,
    disabled: bool,
) -> (&'m mut CreateReply<'att>, usize) {
    let mut queue: VecDeque<_> = queue.iter().enumerate().collect();
    let total_pages = (queue.len() as f64 / PAGE_SIZE as f64).ceil() as usize;
    let (_, np) = queue.pop_front().unwrap();

    let page = page.clamp(0, total_pages - 1);

    let m = m
        .embed(|e| create_queue_embed(e, np, &queue, page, total_pages))
        .components(|c| create_queue_components(c, page, total_pages, disabled));

    (m, page)
}

pub(crate) fn queue_message_edit<'m, 'att>(
    m: &'m mut EditMessage<'att>,
    queue: &[TrackHandle],
    page: usize,
) -> (&'m mut EditMessage<'att>, usize) {
    let mut queue: VecDeque<_> = queue.iter().enumerate().collect();
    let total_pages = (queue.len() as f32 / PAGE_SIZE as f32).ceil() as usize;
    let (_, np) = queue.pop_front().unwrap();

    let page = page.clamp(0, total_pages - 1);

    let m = m
        .embed(|e| create_queue_embed(e, np, &queue, page, total_pages))
        .components(|c| create_queue_components(c, page, total_pages, false));

    (m, page)
}

pub(crate) fn format_user_for_log(user: &User) -> String {
    format!("{} [{}]", user.tag(), user.id)
}
