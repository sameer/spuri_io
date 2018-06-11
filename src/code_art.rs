use actix_web::{HttpResponse, Responder, State};
use askama::Template;
use base::*;
use std::sync::Arc;

#[derive(Template)]
#[template(path = "code_art_gallery.html")]
pub struct CodeArtGallery<'a> {
    pub _parent: Arc<Base<'a>>,
    pub images: Vec<CodeArtImage>,
}

pub fn code_art_gallery(state: State<CodeArtGallery>) -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(state.render().unwrap())
}

pub struct CodeArtImage {
    name: String,
    href: String,
    src: String,
    desc: String,
}
