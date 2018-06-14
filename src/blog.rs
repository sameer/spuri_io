use actix_web::{HttpResponse, Path, Responder, State};
use askama::Template;
use base::*;
use static_pages::Error;
use std::sync::Arc;

#[derive(Template)]
#[template(path = "blog_page.html")]
pub struct BlogPage {
    _parent: Arc<Base>,
    title: String,
    checksum: String,
    author: String,
    body: String,
    last_modified: String,
}

pub fn blog_page(data: (State<BlogIndex>, Path<String>)) -> impl Responder {
    info!("Hit");
    let path_page_title = data.1.into_inner();
    for page in &data.0.index {
        if page.title == path_page_title {
            return HttpResponse::Ok()
                .content_type("text/html")
                .body(page.render().unwrap());
        }
    }
    info!("fail");
    HttpResponse::NotFound().content_type("text/html").body(
        Error {
            _parent: data.0._parent.clone(),
            title: "Blog Page Not Found".to_string(),
            msg: "The blog page you requested was not found".to_string(),
        }.render()
            .unwrap(),
    )
}

pub fn blog_index(data: State<BlogIndex>) -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(data.render().unwrap())
}

#[derive(Template)]
#[template(path = "blog_index.html")]
pub struct BlogIndex {
    pub _parent: Arc<Base>,
    pub index: Vec<BlogPage>,
}
