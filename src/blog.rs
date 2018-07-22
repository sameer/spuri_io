use actix_web::http::header::ContentType;
use actix_web::{HttpResponse, Path, Responder, State};
use askama::Template;
use base::*;
use err::NotFound;
use std::sync::{Arc, RwLock};

#[derive(Template)]
#[template(path = "blog_page.html")]
struct Post {
    _parent: Arc<Base>,
    title: String,
    checksum: String,
    author: String,
    body: String,
    last_modified: String,
}

#[derive(Template)]
#[template(path = "blog_index.html")]
pub struct Index {
    _parent: Arc<Base>,
    index: Vec<Post>,
}

impl Index {
    pub fn get_index(state: State<Arc<RwLock<Index>>>) -> impl Responder {
        HttpResponse::Ok()
            .set(ContentType::html())
            .body(state.read().unwrap().render().unwrap())
    }

    pub fn get_page((state, path): (State<Arc<RwLock<Index>>>, Path<String>)) -> impl Responder {
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
                    NotFound {
                        _parent: state._parent.clone(),
                        title: "Blog Page Not Found".to_string(),
                        msg: "The blog page you requested was not found".to_string(),
                    }.render()
                        .unwrap(),
                )
            })
    }

    pub fn new(parent: Arc<Base>) -> Arc<RwLock<Index>> {
        Arc::new(RwLock::new(Index::from(parent)))
    }
}

impl From<Arc<Base>> for Index {
    fn from(parent: Arc<Base>) -> Index {
        Index {
            _parent: parent,
            index: Vec::new(),
        }
    }
}
