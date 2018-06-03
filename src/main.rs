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
use std::fs::File;
use std::io::prelude::*;

struct NavItem {
    link: String,
    name: String,
    new_page: bool,
}

impl NavItem {
    fn new(link: &str, name: &str) -> NavItem {
        NavItem {
            link: link.to_string(),
            name: name.to_string(),
            new_page: false,
        }
    }
}

impl Clone for NavItem {
    fn clone(&self) -> NavItem {
        NavItem {
            link: self.link.clone(),
            name: self.name.clone(),
            new_page: self.new_page.clone(),
        }
    }
}

#[derive(Template)]
#[template(path = "base.html")]
struct Base {
    nav_items: Vec<NavItem>,
    css_file_hash: String,
}

impl Clone for Base {
    fn clone(&self) -> Base {
        Base {
            nav_items: self.nav_items.clone(),
            css_file_hash: self.css_file_hash.clone(),
        }
    }
}

fn make_base() -> Base {
    info!("Getting style.css hash");
    let hash = match File::open("static/style.css") {
        Ok(mut f) => {
            let mut hasher = openssl::sha::Sha512::new();
            let mut buf = vec![];
            match f.read_to_end(&mut buf) {
                Ok(_) => {
                    hasher.update(&buf);
                    let hash: &[u8] = &hasher.finish();
                    base64::encode(hash)
                }
                Err(_) => "".to_string(),
            }
        }
        Err(_) => "".to_string(),
    };
    info!("It is {}", hash);
    return Base {
        nav_items: vec![
            NavItem::new("/code_art", "Code Art"),
            NavItem::new("/blog", "Blog"),
            NavItem::new("https://github.com/sameer", "GitHub"),
            NavItem::new("/about", "About"),
        ],
        css_file_hash: format!("sha512-{}", hash),
    };
}

#[derive(Template)]
#[template(path = "about.html")]
struct About<'a> {
    _parent: &'a Base,
}

fn about(req: HttpRequest<Base>) -> impl Responder {
    HttpResponse::Ok().body(
        About {
            _parent: req.state(),
        }.render()
            .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "index.html")]
struct Index<'a> {
    _parent: &'a Base,
}

fn index(req: HttpRequest<Base>) -> impl Responder {
    HttpResponse::Ok().content_type("text/html").body(
        Index {
            _parent: req.state(),
        }.render()
            .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "error.html")]
struct Error<'a> {
    _parent: &'a Base,
    title: String,
    msg: String,
}

#[derive(Template)]
#[template(path = "blog_page.html")]
struct BlogPage<'a> {
    _parent: &'a Base,
    title: String,
    checksum: String,
    author: String,
    body: String,
    last_modified: String,
}

fn blog_page(req: HttpRequest<Base>) -> impl Responder {
    req.
}

#[derive(Template)]
#[template(path = "blog_index.html")]
struct BlogIndex<'a> {
    _parent: &'a Base,
    title: String,
    index: Vec<BlogPage<'a>>,
}

#[derive(Template)]
#[template(path = "codeart_gallery.html")]
struct CodeArtGallery<'a> {
    _parent: &'a Base,
    images: Vec<CodeArtImage>,
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
    let base = make_base();
    let serv = server::new(move || {
        App::with_state(base.clone())
            .handler("/static", fs::StaticFiles::new("./static"))
            .resource("/", |r| r.f(index))
            .resource("/about", |r| r.f(about))
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
