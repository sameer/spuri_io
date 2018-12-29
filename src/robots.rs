use actix_web::http::header::ContentType;
use actix_web::{HttpRequest, HttpResponse, Responder};
use base::Base;
use std::sync::Arc;

const ROBOTS_TXT: &str = "User-agent: *
Disallow: /files";

pub fn get_robots_txt(_state: &HttpRequest<Arc<Base>>) -> impl Responder {
    HttpResponse::Ok().set(ContentType::plaintext()).body(ROBOTS_TXT)
}
