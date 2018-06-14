use actix_web::{HttpResponse, Responder, State};
use askama::Template;
use base::*;
use notify::{watcher, DebouncedEvent, RecursiveMode, Watcher};
use std::env;
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

#[derive(Template)]
#[template(path = "code_art_gallery.html")]
pub struct Gallery {
    pub _parent: Arc<Base>,
    pub images: Vec<Image>,
}

pub fn gallery(state: State<Arc<RwLock<Gallery>>>) -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(state.read().unwrap().render().unwrap())
}

pub fn spawn_gallery_updater(gallery_state: Arc<RwLock<Gallery>>) {
    thread::spawn(move || {
        let (tx, rx) = channel();
        let mut watcher = watcher(tx, Duration::from_secs(10)).unwrap();
        watcher
            .watch("./files/code_art", RecursiveMode::NonRecursive)
            .unwrap();
        loop {
            match rx.recv() {
                Ok(event) => match event {
                    DebouncedEvent::Rename(from_path, to_path) => {
                        let mut state = gallery_state.write().unwrap();
                        state
                            .images
                            .iter()
                            .position(|ref img| from_path.ends_with(PathBuf::from(img.src.clone())))
                            .and_then(|position: usize| {
                                state.images.get_mut(position).and_then(|img| {
                                    image_path_to_src(&to_path)
                                        .and_then(|src| {
                                            img.src = src;
                                            image_path_to_name(&to_path)
                                        })
                                        .and_then(|name| {
                                            img.name = name;
                                            image_path_to_desc(&to_path)
                                        })
                                        .and_then(|desc| {
                                            img.desc = desc;
                                            Some(true)
                                        })
                                })
                            });
                    }
                    DebouncedEvent::Create(path) => {
                        if let (Some(src), Some(name), Some(desc)) = (
                            image_path_to_src(&path),
                            image_path_to_name(&path),
                            image_path_to_desc(&path),
                        ) {
                            let image = Image {
                                href: String::from("#"),
                                src: src,
                                name: name,
                                desc: desc,
                            };
                            gallery_state.write().unwrap().images.push(image);
                        }
                    }
                    _ => {}
                },
                Err(err) => error!("{}", err),
            }
        }
    });
}

fn image_path_to_src(path: &PathBuf) -> Option<String> {
    env::current_dir().ok().and_then(|cwd_path_buf| {
        let gallery_prefix = cwd_path_buf.join(PathBuf::from("./files/"));
        path.strip_prefix(gallery_prefix)
            .ok()
            .and_then(|stripped| stripped.to_str())
            .and_then(|stripped_str| Some(stripped_str.to_string()))
    })
}

fn image_path_to_name(path: &PathBuf) -> Option<String> {
    path.file_name()
        .and_then(|os_str| os_str.to_str())
        .and_then(|regular_str| Some(regular_str.to_string()))
}

// TODO: find a way to store descriptions directly corresponding to the images
fn image_path_to_desc(_path: &PathBuf) -> Option<String> {
    Some(String::new())
}

#[derive(Clone)]
pub struct Image {
    name: String,
    href: String,
    src: String,
    desc: String,
}
