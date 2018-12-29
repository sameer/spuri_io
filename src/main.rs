#![feature(try_from)]
extern crate actix;
extern crate actix_web;
#[macro_use]
extern crate askama;
extern crate openssl;
#[macro_use]
extern crate log;
extern crate base64;
extern crate env_logger;
extern crate notify;
#[macro_use]
extern crate serde_derive;
extern crate ammonia;
extern crate chrono;
extern crate image;
extern crate pulldown_cmark;
extern crate serde_urlencoded;

use actix_web::http::header::IntoHeaderValue;
use actix_web::{fs, http, middleware, server, App};
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use std::env;
use std::sync::Arc;

mod base;
use base::*;
mod blog;
mod code_art;
mod err;
mod header;
mod robots;
mod static_pages;

const DEV_BIND_ADDRESS: &str = "127.0.0.1:8080";

fn main() {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info,spuri_io=debug,actix_web=info");
    }
    env_logger::init();
    info!("Starting...");

    let sys = actix::System::new("template-askama");
    let bind_address: String = env::var("PROD_BIND_ADDRESS").unwrap_or_else(|_err| {
        warn!("No production address found, defaulting to development address.");
        DEV_BIND_ADDRESS.to_string()
    });

    let base_arc = Arc::new(BASE);
    let blog_index = blog::Blog::new(base_arc.clone());
    let code_art_gallery = code_art::Gallery::new(base_arc.clone());

    let serv = server::new(move || {
        // TODO: find a way to do this on the fly rather than doing it in
        // an ugly manner here
        vec![
            App::with_state(blog_index.clone())
                .middleware(middleware::Logger::default())
                .prefix("/blog")
                .resource("/", |r| r.with(blog::Blog::get_index))
                .resource("/{page}", |r| r.with(blog::Blog::get_post))
                .boxed(),
            App::with_state(code_art_gallery.clone())
                .middleware(middleware::Logger::default())
                .prefix("/code_art")
                .resource("/", |r| r.with(code_art::Gallery::get_index))
                .resource("/resizer", |r| r.with(code_art::Gallery::get_resizer))
                .boxed(),
            App::new()
                .middleware(middleware::Logger::default())
                .middleware(middleware::DefaultHeaders::new().header(
                    http::header::CACHE_CONTROL,
                    header::cache_for_one_day().try_into().unwrap(),
                ))
                .prefix("/files")
                .handler("/", fs::StaticFiles::new("./files").unwrap())
                .boxed(),
            App::with_state(base_arc.clone())
                .middleware(middleware::Logger::default())
                .resource("/", |r| r.f(static_pages::Index::get))
                .resource("/about", |r| r.f(static_pages::About::get))
                .resource("/robots.txt", |r| r.f(robots::get_robots_txt))
                .boxed(),
        ]
    });

    let trying_to_be_secure: bool = bind_address.ends_with(":443");
    if trying_to_be_secure {
        let cert_file_path = env::var("CERT_FILE").unwrap();
        let key_file_path = env::var("KEY_FILE").unwrap();
        let mut ssl_acceptor_builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
        ssl_acceptor_builder
            .set_certificate_file(cert_file_path, SslFiletype::PEM)
            .unwrap();
        ssl_acceptor_builder
            .set_private_key_file(key_file_path, SslFiletype::PEM)
            .unwrap();
        info!("Running in SSL mode");
        serv.bind_ssl(bind_address, ssl_acceptor_builder)
            .unwrap()
            .start();
    } else {
        warn!("Running insecurely!");
        serv.bind(bind_address).unwrap().start();
    }
    info!("Ready!");
    let _ = sys.run();
}
