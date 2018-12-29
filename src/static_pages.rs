use actix_web::http::header::ContentType;
use actix_web::{HttpRequest, HttpResponse, Responder};
use askama::Template;
use base::*;
use header::cache_for_one_week;
use std::sync::Arc;

#[derive(Template)]
#[template(path = "about.html")]
pub struct About {
    _parent: Arc<Base>,
}

impl About {
    pub fn get(req: &HttpRequest<Arc<Base>>) -> impl Responder {
        HttpResponse::Ok()
            .set(cache_for_one_week())
            .set(ContentType::html())
            .body(
                About {
                    _parent: req.state().clone(),
                }.render()
                    .unwrap(),
            )
    }
}

#[derive(Template)]
#[template(path = "index.html")]
pub struct Index {
    _parent: Arc<Base>,
}

impl Index {
    pub fn get(req: &HttpRequest<Arc<Base>>) -> impl Responder {
        HttpResponse::Ok()
            .set(cache_for_one_week())
            .set(ContentType::html())
            .body(
                Index {
                    _parent: req.state().clone(),
                }.render()
                    .unwrap(),
            )
    }
}
