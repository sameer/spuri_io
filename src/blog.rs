use actix_web::http::header::ContentType;
use actix_web::{HttpResponse, Path, Responder, State};
use askama::Template;
use base::*;
use static_pages::Error;
use std::sync::{Arc, RwLock};

#[derive(Template)]
#[template(path = "blog_page.html")]
struct BlogPage {
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
    pub fn get_index(state: State<Arc<RwLock<BlogIndex>>>) -> impl Responder {
        HttpResponse::Ok()
            .set(ContentType::html())
            .body(state.read().unwrap().render().unwrap())
    }

    pub fn get_page((state, path): (State<Arc<RwLock<BlogIndex>>>, Path<String>)) -> impl Responder {
        let (state, path) = (state.read().unwrap(), path.into_inner());
        state
            .index
            .iter()
            .find(|page| page.title == path)
            .map(|page| {
                HttpResponse::Ok()
                    .set(ContentType::html())
                    .body(page.render().unwrap())
            })
            .unwrap_or_else(|| {
                HttpResponse::NotFound().set(ContentType::html()).body(
                    Error {
                        _parent: state._parent.clone(),
                        title: "Blog Page Not Found".to_string(),
                        msg: "The blog page you requested was not found".to_string(),
                    }.render()
                        .unwrap(),
                )
            })
    }

    pub fn new(parent: Arc<Base>) -> Arc<RwLock<BlogIndex>> {
        Arc::new(RwLock::new(BlogIndex::from(parent)))
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
