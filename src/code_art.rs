use actix_web::{HttpResponse, Responder, State};
use askama::Template;
use base::*;
use notify::{watcher, DebouncedEvent, RecursiveMode, Watcher};
use std::convert::TryFrom;
use std::env;
use std::error;
use std::fs;
use std::path::PathBuf;
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

impl Gallery {
    fn remove_image(&mut self, path: &PathBuf) {
        match image_path_to_src(&path) {
            Ok(old_src) => {
                self.images
                    .iter()
                    .position(|ref img| img.src == old_src)
                    .map(|pos: usize| self.images.swap_remove(pos))
                    .and_then(|old_img| {
                        debug!("Removing image {}", old_img.name);
                        Some(true)
                    });
            }
            Err(err) => warn!("Couldn't derive original image by path: {}", err),
        }
    }
}

pub fn gallery(state: State<Arc<RwLock<Gallery>>>) -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(state.read().unwrap().render().unwrap())
}

const FOLDER_PATH: &str = "./files/code_art";

pub fn spawn_gallery_updater(gallery_state: Arc<RwLock<Gallery>>) {
    thread::spawn(move || {
        match env::current_dir().and_then(|cwd_path_buf| {
            let gallery_prefix = cwd_path_buf.join(PathBuf::from(FOLDER_PATH));
            fs::read_dir(gallery_prefix)
        }) {
            Ok(dir_iter) => {
                let mut state = gallery_state.write().unwrap();
                let length_before = state.images.len();
                let path_iter = dir_iter.map(|dir_entry_result| {
                    dir_entry_result.and_then(|dir_entry| Ok(dir_entry.path()))
                });
                path_iter.for_each(|path_result| {
                    match path_result {
                        Ok(path) => match Image::try_from(&path) {
                            Ok(img) => {
                                debug!("Adding image {}", img.name);
                                state.images.push(img);
                            }
                            Err(err) => warn!("Couldn't derive new image by path: {}", err),
                        },
                        Err(err) => {
                            warn!(
                                "Error while reading file from directory: {}",
                                err
                            );
                        }
                    };
                });
                let length_after = state.images.len();
                info!("Found {} images", length_after - length_before);
            }
            Err(err) => {
                error!("Error while reading files from directory: {}", err);
            }
        };
        let (tx, rx) = channel();
        let mut watcher = watcher(tx, Duration::from_secs(2)).unwrap();
        match watcher.watch(FOLDER_PATH, RecursiveMode::Recursive) {
            Ok(()) => {}
            Err(err) => {
                error!("Could not watch directory: {}", err);
            }
        }
        loop {
            match rx.recv() {
                Ok(event) => match event {
                    DebouncedEvent::Rename(from_path, to_path) => match Image::try_from(&to_path) {
                        Ok(img_to_add) => {
                            let mut state = gallery_state.write().unwrap();
                            state.remove_image(&from_path);
                            debug!("Handling new image {}", img_to_add.name);
                            state.images.push(img_to_add);
                        }
                        Err(err) => warn!("Couldn't derive moved image by path: {}", err),
                    },
                    DebouncedEvent::Create(path) => match Image::try_from(&path) {
                        Ok(img) => {
                            debug!("Handling new image {}", img.name);
                            gallery_state.write().unwrap().images.push(img);
                        }
                        Err(err) => warn!("Couldn't derive new image by path: {}", err),
                    },
                    DebouncedEvent::Remove(path) => {
                        let mut state = gallery_state.write().unwrap();
                        state.remove_image(&path);
                    }
                    _ => {}
                },
                Err(err) => error!("{}", err),
            }
        }
    });
}

#[derive(Clone)]
pub struct Image {
    name: String,
    href: String,
    src: String,
    desc: String,
}

impl<'a> TryFrom<&'a PathBuf> for Image {
    type Error = Box<error::Error>;
    fn try_from(path: &PathBuf) -> Result<Self, Self::Error> {
        match (
            image_path_to_src(path),
            image_path_to_name(path),
            image_path_to_desc(path),
        ) {
            (Ok(src), Some(name), Some(desc)) => Ok(Image {
                href: String::from("#"),
                src: src,
                name: name,
                desc: desc,
            }),
            (Err(err), _, _) => Err(err),
            _ => Err(From::from("one or more Os Strings did not contain unicode")),
        }
    }
}

fn image_path_to_src(path: &PathBuf) -> Result<String, Box<error::Error>> {
    match env::current_dir() {
        Ok(cwd_path_buf) => match path.strip_prefix(cwd_path_buf) {
            Ok(stripped) => Ok(PathBuf::from("/")
                .join(stripped)
                .to_str()
                .unwrap_or_else(|| "")
                .to_string()),
            Err(err) => Err(Box::new(err)),
        },
        Err(err) => Err(Box::new(err)),
    }
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
