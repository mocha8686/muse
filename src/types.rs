pub(crate) struct Data;
pub(crate) type Error = anyhow::Error;
pub(crate) type Context<'a> = poise::Context<'a, Data, Error>;
pub(crate) type FrameworkError<'a> = poise::FrameworkError<'a, Data, Error>;
