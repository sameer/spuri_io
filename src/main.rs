extern crate actix;
extern crate actix_web;
#[macro_use]
extern crate askama;
extern crate openssl;
#[macro_use]
extern crate log;
extern crate base64;
extern crate env_logger;

use actix_web::*;
use askama::Template;
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use std::env;

struct NavItem<'a> {
    link: &'a str,
    name: &'a str,
    new_page: bool,
}

impl<'a> Clone for NavItem<'a> {
    fn clone(&self) -> NavItem<'a> {
        NavItem {
            link: self.link,
            name: self.name,
            new_page: self.new_page,
        }
    }
}

#[derive(Template)]
#[template(path = "base.html")]
struct Base<'a> {
    nav_items: [NavItem<'a>; 4],
    css_file_hash: &'a str,
}

const BASE: Base = Base {
    css_file_hash: include_str!("css_file_hash"),
    nav_items: [
        NavItem {
            link: "/code_art",
            name: "Code Art",
            new_page: false,
        },
        NavItem {
            link: "/blog",
            name: "Blog",
            new_page: false,
        },
        NavItem {
            link: "https://github.com/sameer",
            name: "GitHub",
            new_page: false,
        },
        NavItem {
            link: "/about",
            name: "About",
            new_page: false,
        },
    ],
};

impl<'a> Clone for Base<'a> {
    fn clone(&self) -> Base<'a> {
        Base {
            nav_items: self.nav_items.clone(),
            css_file_hash: self.css_file_hash,
        }
    }
}

#[derive(Template)]
#[template(path = "about.html")]
struct About<'a> {
    _parent: Base<'a>,
}

fn about(req: HttpRequest<Base>) -> impl Responder {
    HttpResponse::Ok().body(
        About {
            _parent: req.state().clone(),
        }.render()
            .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "index.html")]
struct Index<'a> {
    _parent: Base<'a>,
}

fn index(req: HttpRequest<Base>) -> impl Responder {
    HttpResponse::Ok().content_type("text/html").body(
        Index {
            _parent: req.state().clone(),
        }.render()
            .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "error.html")]
struct Error<'a> {
    _parent: Base<'a>,
    title: String,
    msg: String,
}

#[derive(Template)]
#[template(path = "blog_page.html")]
struct BlogPage<'a> {
    _parent: Base<'a>,
    title: String,
    checksum: String,
    author: String,
    body: String,
    last_modified: String,
}

fn blog_page(data: (State<BlogIndex>, Path<String>)) -> impl Responder {
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

fn blog_index(data: State<BlogIndex>) -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(data.render().unwrap())
}

#[derive(Template)]
#[template(path = "blog_index.html")]
struct BlogIndex<'a> {
    _parent: Base<'a>,
    index: Vec<BlogPage<'a>>,
}

#[derive(Template)]
#[template(path = "code_art_gallery.html")]
struct CodeArtGallery<'a> {
    _parent: Base<'a>,
    images: Vec<CodeArtImage>,
}

fn code_art_gallery(state: State<CodeArtGallery>) -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(state.render().unwrap())
}

struct CodeArtImage {
    name: String,
    href: String,
    src: String,
    desc: String,
}

const DEV_BIND_ADDRESS: &str = "127.0.0.1:8080";

fn main() {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info,actix_web=info");
    }
    env_logger::init();
    info!("Launching...");

    let sys = actix::System::new("template-askama");
    let bind_address: String =
        env::var("PROD_BIND_ADDRESS").unwrap_or(DEV_BIND_ADDRESS.to_string());
    let cert_file_path = env::var("CERT_FILE").unwrap_or(String::new());
    let key_file_path = env::var("KEY_FILE").unwrap_or(String::new());
    let serv = server::new(move || {
        vec![
            App::with_state(BlogIndex {
                _parent: BASE.clone(),
                index: vec![],
            }).prefix("/blog")
                .resource("/", |r| r.with(blog_index))
                .resource("/{page}", |r| r.with(blog_page))
                .boxed(),
            App::with_state(CodeArtGallery {
                _parent: BASE.clone(),
                images: vec![],
            }).prefix("/code_art")
                .resource("/", |r| r.with(code_art_gallery))
                .boxed(),
            App::with_state(BASE.clone())
                .handler("/static", fs::StaticFiles::new("./static"))
                .resource("/", |r| r.f(index))
                .resource("/about", |r| r.f(about))
                .boxed(),
        ]
    });

    let trying_to_be_secure: bool =
        bind_address.ends_with(":443") && !cert_file_path.is_empty() && !key_file_path.is_empty();
    match trying_to_be_secure {
        true => {
            info!("Running in SSL mode");
            let mut ssl_acceptor_builder =
                SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
            ssl_acceptor_builder
                .set_certificate_file(cert_file_path, SslFiletype::PEM)
                .unwrap();
            ssl_acceptor_builder
                .set_private_key_file(key_file_path, SslFiletype::PEM)
                .unwrap();
            serv.bind_ssl(bind_address, ssl_acceptor_builder)
                .unwrap()
                .start()
        }
        false => serv.bind(bind_address).unwrap().start(),
    };
    info!("Launched!");
    let _ = sys.run();
}
