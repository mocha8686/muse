use poise::serenity_prelude::{CreateEmbed, User};
use songbird::input::Metadata;

pub(crate) fn base_embed(e: &mut CreateEmbed) -> &mut CreateEmbed {
    e.color(0x0789f0)
}

pub(crate) fn song_embed<'e, 's>(
    e: &'e mut CreateEmbed,
    song: &'s Metadata,
) -> &'e mut CreateEmbed {
    todo!()
}

pub(crate) fn format_user_for_log(user: &User) -> String {
    format!("{} [{}]", user.tag(), user.id)
}
