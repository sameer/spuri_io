use askama::Template;
use std::sync::Arc;
use base::*;

#[derive(Template)]
#[template(path = "error.html")]
pub struct NotFound {
    pub _parent: Arc<Base>,
    pub title: String,
    pub msg: String,
}
