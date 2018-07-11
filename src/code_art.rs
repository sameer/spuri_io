use actix_web::http::StatusCode;
use actix_web::{HttpResponse, Query, Responder, State};
use askama::Template;
use base::*;
use image::{FilterType, ImageOutputFormat, ImageResult};
use notify::{watcher, DebouncedEvent, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::env;
use std::error;
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

// Sizes are those denoted to be 16:9/~16:9 in the range of (N/A, 100%] percent of web users by https://en.wikipedia.org/wiki/Display_resolution
// Of note (to future me) is the fact that 4K UHD/WQHD is not included; my code art images are never larger than FHD (yet), so there is no reason
// to provide these sizes.
const AVAILABLE_SIZES: [(u32, u32); 6] = [
    (1920, 1080),
    (1600, 900),
    (1536, 864),
    (1366, 768),
    (1360, 768),
    (1280, 720),
];

pub fn resizer(
    (gallery_state, resizer_query): (State<Arc<RwLock<Gallery>>>, Query<Resize>),
) -> impl Responder {
    let size_tuple = (resizer_query.width, resizer_query.height);
    if AVAILABLE_SIZES.iter().any(|size| size == &size_tuple) {
        gallery_state
            .read()
            .unwrap()
            .images
            .iter()
            .find(|img| img.src == resizer_query.src)
            .map_or_else(
                || HttpResponse::NotFound().finish(),
                |img| {
                    img.size_to_image_bytes.get(&size_tuple).map_or_else(
                        || HttpResponse::InternalServerError().finish(),
                        |resized_image_bytes| {
                            HttpResponse::Ok()
                                .content_type("image/png")
                                .body(resized_image_bytes)
                        },
                    )
                },
            )
    }
    // This is the ideal response code here. The query is valid and well formed but it will not be processed because it doesn't match the
    // expectation that it should be one of AVAILABLE_SIZES. If this happens, in all likelihood, someone is just messing around with the query.
    else {
        HttpResponse::build(StatusCode::UNPROCESSABLE_ENTITY).finish()
    }
}

pub fn gallery(state: State<Arc<RwLock<Gallery>>>) -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(state.read().unwrap().render().unwrap())
}

#[derive(Template)]
#[template(path = "code_art_gallery.html")]
pub struct Gallery {
    _parent: Arc<Base>,
    images: Vec<Image>,
}

impl Gallery {
    fn remove_image(&mut self, path_to_remove: &PathBuf) -> Result<Image, Box<error::Error>> {
        Image::path_to_src(&path_to_remove).and_then(|src_to_remove| {
            self.images
                .iter()
                .position(|ref img| img.src == src_to_remove)
                .map(|pos_to_remove: usize| self.images.swap_remove(pos_to_remove))
                .ok_or_else(|| From::from("Could not find old image by src"))
        })
    }

    fn initialize(&mut self) {
        match env::current_dir().and_then(|cwd_path_buf| {
            let gallery_prefix = cwd_path_buf.join(PathBuf::from(FOLDER_PATH));
            fs::read_dir(gallery_prefix)
        }) {
            Ok(dir_iter) => {
                let length_before = self.images.len();
                let path_iter = dir_iter
                    .map(|dir_entry_result| dir_entry_result.map(|dir_entry| dir_entry.path()));
                path_iter.for_each(|path_result| {
                    match path_result {
                        Ok(path) => match Image::try_from(&path) {
                            Ok(img) => {
                                debug!("Adding {:?}", img);
                                self.images.push(img);
                            }
                            Err(err) => warn!("Couldn't derive new image by path: {}", err),
                        },
                        Err(err) => {
                            warn!("Error while reading file from directory: {}", err);
                        }
                    };
                });
                let length_after = self.images.len();
                info!("Found {} images", length_after - length_before);
            }
            Err(err) => {
                error!("Error while reading files from directory: {}", err);
            }
        };
    }

    pub fn new(parent: Arc<Base>) -> Arc<RwLock<Gallery>> {
        let gallery = Arc::new(RwLock::new(Gallery {
            _parent: parent,
            images: Vec::new(),
        }));
        spawn_gallery_updater(gallery.clone());
        gallery
    }
}

#[derive(Deserialize, Serialize)]
pub struct Resize {
    width: u32,
    height: u32,
    src: String,
}

const FOLDER_PATH: &str = "./files/code_art";
fn spawn_gallery_updater(gallery_state: Arc<RwLock<Gallery>>) {
    thread::spawn(move || {
        gallery_state.write().unwrap().initialize();

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
struct Image {
    name: String,
    href: String,
    srcset: String,
    src: String,
    desc: String,
    size_to_image_bytes: HashMap<(u32, u32), Arc<Vec<u8>>>,
}

impl Image {
    fn path_to_src(path: &PathBuf) -> Result<String, Box<error::Error>> {
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

    fn src_to_srcset(src: &String) -> String {
        // AVAILABLE_SIZES
        AVAILABLE_SIZES
            .iter()
            .map(|size| Resize {
                width: size.0,
                height: size.1,
                src: src.clone(),
            })
            .map(|resize| {
                format!(
                    "/code_art/resizer{}",
                    serde_urlencoded::to_string(resize).unwrap()
                )
            })
            .collect::<Vec<String>>()
            .join(" ")
    }

    // Adds a space before uppercase letters excluding the first. 'CamelCaseName' --> 'Camel Case Name'
    fn path_to_name(path: &PathBuf) -> Option<String> {
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
    fn path_to_desc(_path: &PathBuf) -> Option<String> {
        Some(String::new())
    }

    fn path_to_resized_image_bytes(
        path: &PathBuf,
    ) -> ImageResult<HashMap<(u32, u32), Arc<Vec<u8>>>> {
        image::open(path).and_then(|dynamic_image| {
            let mut size_to_image_bytes = HashMap::new();
            AVAILABLE_SIZES
                .iter()
                .try_for_each(|size| {
                    let resized_dynamic_image =
                        dynamic_image.resize_exact(size.0, size.1, FilterType::Nearest);
                    let mut resized_image_bytes = Vec::new();
                    // If this fails, indicates that png_codec is unavailable, in which case
                    let write_result = resized_dynamic_image
                        .write_to(&mut resized_image_bytes, ImageOutputFormat::PNG);

                    if write_result.is_ok() {
                        size_to_image_bytes.insert(size.clone(), Arc::new(resized_image_bytes));
                    }
                    write_result
                })
                .map(|_| size_to_image_bytes)
        })
    }
}

impl<'a> TryFrom<&'a PathBuf> for Image {
    type Error = Box<error::Error>;
    fn try_from(path: &PathBuf) -> Result<Self, Self::Error> {
        match (
            Image::path_to_src(path),
            Image::path_to_name(path),
            Image::path_to_desc(path),
            Image::path_to_resized_image_bytes(path),
        ) {
            (Ok(src), Some(name), Some(desc), Ok(size_to_image_bytes)) => Ok(Image {
                href: src.clone(),
                srcset: Image::src_to_srcset(&src),
                src: src,
                name: name,
                desc: desc,
                size_to_image_bytes: size_to_image_bytes
            }),
            (Err(err), _, _, _) => Err(err),
            (Ok(_), _, _, Err(err)) => Err(Box::new(err)),
            _ => Err(From::from(
                format!("Path contained invalid unicode characters or no file name could be identified: {:?}", path.as_os_str()),
            )),
        }
    }
}
