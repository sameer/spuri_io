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
    fn remove_image(&mut self, path_to_remove: &PathBuf) -> Result<Image, Box<error::Error>> {
        image_path_to_src(&path_to_remove).and_then(|src_to_remove| {
            self.images
                .iter()
                .position(|ref img| img.src == src_to_remove)
                .map(|pos_to_remove: usize| self.images.swap_remove(pos_to_remove))
                .ok_or_else(|| From::from("Could not find old image by src"))
        })
    }
}

pub fn gallery(state: State<Arc<RwLock<Gallery>>>) -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(state.read().unwrap().render().unwrap())
}

const FOLDER_PATH: &str = "./files/code_art";

fn initialize_gallery(uninitialized_gallery_state: Arc<RwLock<Gallery>>) {
    match env::current_dir().and_then(|cwd_path_buf| {
        let gallery_prefix = cwd_path_buf.join(PathBuf::from(FOLDER_PATH));
        fs::read_dir(gallery_prefix)
    }) {
        Ok(dir_iter) => {
            let mut state = uninitialized_gallery_state.write().unwrap();
            let length_before = state.images.len();
            let path_iter = dir_iter.map(|dir_entry_result| {
                dir_entry_result.and_then(|dir_entry| Ok(dir_entry.path()))
            });
            path_iter.for_each(|path_result| {
                match path_result {
                    Ok(path) => match Image::try_from(&path) {
                        Ok(img) => {
                            debug!("Adding {:?}", img);
                            state.images.push(img);
                        }
                        Err(err) => warn!("Couldn't derive new image by path: {}", err),
                    },
                    Err(err) => {
                        warn!("Error while reading file from directory: {}", err);
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
}

pub fn spawn_gallery_updater(gallery_state: Arc<RwLock<Gallery>>) {
    thread::spawn(move || {
        initialize_gallery(gallery_state.clone());

        let (tx, notify_event_receiver) = channel();
        let mut watcher = watcher(tx, Duration::from_secs(2)).unwrap();
        match watcher.watch(FOLDER_PATH, RecursiveMode::Recursive) {
            Ok(()) => {}
            Err(err) => {
                error!("Could not watch code art directory: {}", err);
                return;
            }
        }
        loop {
            match notify_event_receiver.recv() {
                Ok(event) => match event {
                    DebouncedEvent::Rename(original_path, renamed_path) => {
                        match Image::try_from(&renamed_path) {
                            Ok(renamed_img) => {
                                let mut state = gallery_state.write().unwrap();
                                state
                                    .remove_image(&original_path)
                                    .and_then(|original_img| {
                                        debug!(
                                            "Handling move from {:?} to {:?}",
                                            original_img, renamed_img
                                        );
                                        Ok(original_img)
                                    })
                                    .unwrap();
                                state.images.push(renamed_img);
                            }
                            Err(err) => warn!("Couldn't derive moved image by path: {}", err),
                        }
                    }
                    DebouncedEvent::Create(created_path) => match Image::try_from(&created_path) {
                        Ok(created_img) => {
                            debug!("Handling added {:?}", created_img);
                            gallery_state.write().unwrap().images.push(created_img);
                        }
                        Err(err) => warn!("Couldn't derive new image by path: {}", err),
                    },
                    DebouncedEvent::Remove(removed_path) => {
                        let mut state = gallery_state.write().unwrap();
                        state
                            .remove_image(&removed_path)
                            .and_then(|img| {
                                debug!("Handling removed {:?}", img);
                                Ok(img)
                            })
                            .unwrap();
                    }
                    _ => {}
                },
                Err(err) => error!("Encountered error in notify event loop {}", err),
            }
        }
    });
}

#[derive(Clone, Debug)]
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
                href: src.clone(),
                src: src, // TODO: change src to be a srcset with smaller alternatives
                name: name,
                desc: desc,
            }),
            (Err(err), _, _) => Err(err),
            _ => Err(From::from(
                format!("Path contained invalid unicode characters or no file name could be identified: {:?}", path.as_os_str()),
            )),
        }
    }
}

fn image_path_to_src(path: &PathBuf) -> Result<String, Box<error::Error>> {
    match env::current_dir() {
        Ok(cwd_path_buf) => match path.strip_prefix(cwd_path_buf) {
            Ok(relative_path) => PathBuf::from("/")
                .join(relative_path)
                .to_str()
                .map(|s| s.to_string())
                .ok_or_else(|| {
                    From::from(format!(
                        "Path contained invalid unicode characters: {:?}",
                        path.as_os_str()
                    ))
                }),
            Err(err) => Err(Box::new(err)),
        },
        Err(err) => Err(Box::new(err)),
    }
}

// Adds a space before uppercase letters excluding the first. 'CamelCaseName' --> 'Camel Case Name'
fn image_path_to_name(path: &PathBuf) -> Option<String> {
    path.file_stem()
        .and_then(|stem_os_str| stem_os_str.to_str())
        .and_then(|stem_str| Some(stem_str.to_string()))
        .and_then(|stem_string| {
            Some(stem_string.chars().fold(String::new(), |mut acc, x| {
                if acc.len() != 0 && x.is_uppercase() {
                    acc.push(' ');
                }
                acc.push(x);
                acc
            }))
        })
}

// TODO: find a way to store descriptions directly corresponding to the images
fn image_path_to_desc(_path: &PathBuf) -> Option<String> {
    Some(String::new())
}
