use chrono::NaiveDate;
use poise::{
    serenity_prelude::{CreateEmbed, User},
    CreateReply,
};
use songbird::input::Metadata;

pub(crate) fn base_embed(e: &mut CreateEmbed) -> &mut CreateEmbed {
    e.color(0x0789f0)
}

pub(crate) fn song_embed<'e, 'a>(
    mut e: &'e mut CreateEmbed,
    song: &'a Metadata,
) -> &'e mut CreateEmbed {
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
        let secs = duration.as_secs();
        let mins = secs / 60;
        let secs = secs % 60;
        footer.push(format!("[{mins}:{secs:02}]"))
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
        e = e.footer(|f| f.text(footer.join(" • ")))
    }

    e
}

pub(crate) fn now_playing_message<'m, 'att, 'a>(
    mut m: &'m mut CreateReply<'att>,
    song: &'a Metadata,
) -> &'m mut CreateReply<'att> {
    if let Some(title) = &song.title {
        m = m.content(format!("Now playing *{title}*."));
    } else {
        m = m.content(format!("Now playing a new song."));
    }

    m.embed(|e| song_embed(e, song))
}

pub(crate) fn format_user_for_log(user: &User) -> String {
    format!("{} [{}]", user.tag(), user.id)
}
