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

use actix_web::{fs, middleware, server, App};
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use std::env;
use std::sync::{Arc, RwLock};

mod base;
use base::*;

mod code_art;

mod blog;
use blog::*;

mod static_pages;
use static_pages::*;

const DEV_BIND_ADDRESS: &'static str = "127.0.0.1:8080";

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

    let serv = server::new(move || {
        // TODO: find a way to do this on the fly rather than doing it in
        // an ugly manner here
        let base_arc = Arc::new(BASE);
        let code_art_gallery = Arc::new(RwLock::new(code_art::Gallery {
            _parent: base_arc.clone(),
            images: vec![],
        }));
        code_art::spawn_gallery_updater(code_art_gallery.clone());
        vec![
            App::with_state(BlogIndex {
                _parent: base_arc.clone(),
                index: vec![],
            }).middleware(middleware::Logger::default())
                .prefix("/blog")
                .resource("/", |r| r.with(blog_index))
                .resource("/{page}", |r| r.with(blog_page))
                .boxed(),
            App::with_state(code_art_gallery)
                .middleware(middleware::Logger::default())
                .prefix("/code_art")
                .resource("/", |r| r.with(code_art::gallery))
                .boxed(),
            App::with_state(base_arc.clone())
                .middleware(middleware::Logger::default())
                .handler("/files", fs::StaticFiles::new("./files"))
                .resource("/", |r| r.f(index))
                .resource("/about", |r| r.f(about))
                .boxed(),
        ]
    });

    let trying_to_be_secure: bool = bind_address.ends_with(":443");
    match trying_to_be_secure {
        true => {
            let cert_file_path = env::var("CERT_FILE").unwrap();
            let key_file_path = env::var("KEY_FILE").unwrap();
            let mut ssl_acceptor_builder =
                SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
            ssl_acceptor_builder
                .set_certificate_file(cert_file_path, SslFiletype::PEM)
                .unwrap();
            ssl_acceptor_builder
                .set_private_key_file(key_file_path, SslFiletype::PEM)
                .unwrap();
            info!("Running in SSL mode");
            serv.bind_ssl(bind_address, ssl_acceptor_builder)
                .unwrap()
                .start()
        }
        false => {
            warn!("Running insecurely!");
            serv.bind(bind_address).unwrap().start()
        }
    };
    info!("Ready!");
    let _ = sys.run();
}
