pub(crate) struct Data;
pub(crate) type Error = anyhow::Error;
pub(crate) type Context<'a> = poise::Context<'a, Data, Error>;
pub(crate) type FrameworkError<'a> = poise::FrameworkError<'a, Data, Error>;
pub(crate) type Result<T> = anyhow::Result<T>;

pub(crate) const SONGBIRD_MANAGER_ERR: &str = "Failed to acquire Songbird manager.";
pub(crate) const PAGE_SIZE: usize = 5;
