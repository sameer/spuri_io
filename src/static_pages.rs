use actix_web::{HttpRequest, HttpResponse, Responder};
use askama::Template;
use base::*;
use std::sync::Arc;

#[derive(Template)]
#[template(path = "about.html")]
struct About {
    _parent: Arc<Base>,
}

pub fn about(req: HttpRequest<Arc<Base>>) -> impl Responder {
    HttpResponse::Ok().body(
        About {
            _parent: req.state().clone(),
        }.render()
            .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "index.html")]
struct BaseIndex {
    _parent: Arc<Base>,
}

pub fn index(req: HttpRequest<Arc<Base>>) -> impl Responder {
    HttpResponse::Ok().content_type("text/html").body(
        BaseIndex {
            _parent: req.state().clone(),
        }.render()
            .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "error.html")]
pub struct Error {
    pub _parent: Arc<Base>,
    pub title: String,
    pub msg: String,
}
