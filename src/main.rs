#![feature(proc_macro_hygiene, decl_macro)]
#[macro_use]
extern crate rocket;
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
extern crate rocket_contrib;
extern crate serde_urlencoded;

use std::env;
use std::sync::Arc;

mod base;
use base::*;
mod blog;
mod code_art;
mod err;
mod robots;
mod static_pages;

fn main() {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info,spuri_io=debug,actix_web=info");
    }
    env_logger::init();
    info!("Starting...");

    let base_arc = Arc::new(BASE);

    rocket::ignite()
        .manage(BASE)
        .manage(blog::Blog::new(base_arc.clone()))
        .manage(code_art::Gallery::new(base_arc.clone()))
        .mount("/blog", routes![blog::get_index, blog::get_post])
        .mount(
            "/code_art",
            routes![code_art::get_index, code_art::get_resizer],
        )
        .mount(
            "/files",
            rocket_contrib::serve::StaticFiles::new(
                "./files",
                rocket_contrib::serve::Options::None,
            ),
        )
        .mount(
            "/",
            routes![
                static_pages::get_index,
                static_pages::get_about,
                robots::get_robots_txt
            ],
        )
        .launch();
}
