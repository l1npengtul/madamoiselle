use std::sync::Arc;

use poise::serenity_prelude::Http;

use crate::{UserData, error::Error};

pub fn banner_tick(http: Arc<Http>, userdata: Arc<UserData>) -> Result<(), Error> {
    // see if
}
