use actix_web::{HttpResponse, Path, Responder, State};
use askama::Template;
use base::*;
use static_pages::Error;
use std::sync::Arc;

pub fn blog_page(state: (State<Arc<BlogIndex>>, Path<String>)) -> impl Responder {
    let path_page_title = state.1.into_inner();
    for page in &state.0.index {
        if page.title == path_page_title {
            return HttpResponse::Ok()
                .content_type("text/html")
                .body(page.render().unwrap());
        }
    }
    HttpResponse::NotFound().content_type("text/html").body(
        Error {
            _parent: state.0._parent.clone(),
            title: "Blog Page Not Found".to_string(),
            msg: "The blog page you requested was not found".to_string(),
        }.render()
            .unwrap(),
    )
}

pub fn blog_index(state: State<Arc<BlogIndex>>) -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(state.render().unwrap())
}

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

#[derive(Template)]
#[template(path = "blog_index.html")]
pub struct BlogIndex {
    _parent: Arc<Base>,
    index: Vec<BlogPage>,
}

impl BlogIndex {
    pub fn new(parent: Arc<Base>) -> Arc<BlogIndex> {
        Arc::new(BlogIndex::from(parent))
    }
}

impl From<Arc<Base>> for BlogIndex {
    fn from(parent: Arc<Base>) -> BlogIndex {
        BlogIndex {
            _parent: parent,
            index: Vec::new(),
        }
    }
}
