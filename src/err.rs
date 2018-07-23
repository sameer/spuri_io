use askama::Template;
use base::*;
use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Template)]
#[template(path = "error.html")]
pub struct NotFound {
    pub _parent: Arc<Base>,
    pub title: String,
    pub msg: String,
}

pub fn unicode_error(path: &PathBuf) -> Box<Error> {
    From::from(format!(
        "Path contained invalid unicode characters or no file name could be identified: {:?}",
        path.as_os_str()
    ))
}
