use actix_web::http::header::ContentType;
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, Query, Responder, State};
use askama::Template;
use base::*;
use err::NotFound;
use header::cache_forever;
use image::{FilterType, GenericImage, ImageOutputFormat, ImageResult};
use notify::{watcher, DebouncedEvent, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::env;
use std::error;
use std::fmt;
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

// Sizes are all 16:9 of note (to future me) is the fact that 4K UHD/WQHD are not included; my code art images are never larger than FHD (yet), so
// there is no reason to provide these sizes.
type ImageSize = (u32, u32);
const AVAILABLE_SIZES: [ImageSize; 4] = [(1920, 1080), (1280, 720), (960, 540), (640, 360)];

#[derive(Template)]
#[template(path = "code_art_gallery.html", escape = "none")]
pub struct Gallery {
    _parent: Arc<Base>,
    images: Vec<Image>,
}

const FOLDER_PATH: &str = "./files/code_art";
type GalleryState = Arc<RwLock<Gallery>>;
impl Gallery {
    pub fn get_index(state: State<GalleryState>) -> impl Responder {
        HttpResponse::Ok()
            .set(ContentType::html())
            .body(state.read().unwrap().render().unwrap())
    }

    pub fn get_resizer(
        (gallery_state, resizer_query): (State<GalleryState>, Query<Resize>),
    ) -> impl Responder {
        let size_tuple = (resizer_query.width, resizer_query.height);
        if AVAILABLE_SIZES.iter().any(|size| size == &size_tuple) {
            let gallery_state = gallery_state.read().unwrap();
            gallery_state
                .images
                .iter()
                .find(|img| img.src == resizer_query.src)
                .map_or_else(
                    || {
                        HttpResponse::NotFound().set(ContentType::html()).body(
                            NotFound {
                                _parent: gallery_state._parent.clone(),
                                title: "Blog Page Not Found".to_string(),
                                msg: "The blog page you requested was not found".to_string(),
                            }.render()
                                .unwrap(),
                        )
                    },
                    |img| {
                        img.size_to_image_bytes.get(&size_tuple).map_or_else(
                            || HttpResponse::InternalServerError().finish(),
                            |resized_image_bytes| {
                                HttpResponse::Ok()
                                    .set(cache_forever())
                                    .set(ContentType::png())
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

    pub fn new(parent: Arc<Base>) -> GalleryState {
        let gallery = Arc::new(RwLock::new(Gallery::from(parent)));
        Gallery::spawn_updater(gallery.clone());
        gallery
    }

    fn spawn_updater(gallery_state: GalleryState) {
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
                        DebouncedEvent::Create(created_path) => {
                            match Image::try_from(&created_path) {
                                Ok(created_img) => {
                                    debug!("Handling added {:?}", created_img);
                                    gallery_state.write().unwrap().images.push(created_img);
                                }
                                Err(err) => warn!("Couldn't derive new image by path: {}", err),
                            }
                        }
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
}

impl From<Arc<Base>> for Gallery {
    fn from(parent: Arc<Base>) -> Gallery {
        Gallery {
            _parent: parent,
            images: Vec::new(),
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct Resize {
    width: u32,
    height: u32,
    src: String,
}

struct Image {
    name: String,
    href: String,
    srcset: String,
    src: String,
    desc: String,
    size_to_image_bytes: HashMap<ImageSize, Arc<Vec<u8>>>,
}

// TODO: this adds overhead that I feel could be avoided if a wrapper was provided for the size_to_image_bytes field. The wrapper itself would
// implement fmt::Debug and just show something like "not shown" (wording is debatable)
impl fmt::Debug for Image {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Image {{ name: {:?} href: {:?} src: {:?} desc: {:?} }}",
            self.name, self.href, self.src, self.desc
        )
    }
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

    fn src_to_srcset(src: &str) -> String {
        // AVAILABLE_SIZES
        AVAILABLE_SIZES
            .iter()
            .map(|size| Resize {
                width: size.0,
                height: size.1,
                src: src.to_string(),
            })
            .map(|resize| {
                format!(
                    "/code_art/resizer?{} {}w",
                    serde_urlencoded::to_string(&resize).unwrap(),
                    resize.width,
                )
            })
            .collect::<Vec<String>>()
            .join(", ")
    }

    // Adds a space before uppercase letters excluding the first. 'CamelCaseName' --> 'Camel Case Name'
    fn path_to_name(path: &PathBuf) -> Option<String> {
        path.file_stem()
            .and_then(|stem_os_str| stem_os_str.to_str())
            .and_then(|stem_str| Some(stem_str.to_string()))
            .and_then(|stem_string| {
                Some(stem_string.chars().fold(String::new(), |mut acc, x| {
                    if !acc.is_empty() && x.is_uppercase() {
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
    ) -> ImageResult<HashMap<ImageSize, Arc<Vec<u8>>>> {
        image::open(path).and_then(|dynamic_image| {
            let mut size_to_image_bytes = HashMap::new();
            AVAILABLE_SIZES
                .iter()
                .try_for_each(|size| {
                    let resized_dynamic_image =
                        if &(dynamic_image.width(), dynamic_image.height()) == size {
                            dynamic_image.clone()
                        } else {
                            dynamic_image.resize_exact(size.0, size.1, FilterType::Nearest)
                        };
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
                src,
                name,
                desc,
                size_to_image_bytes,
            }),
            (Err(err), _, _, _) => Err(err),
            (Ok(_), _, _, Err(err)) => Err(Box::new(err)),
            _ => Err(From::from(
                format!("Path contained invalid unicode characters or no file name could be identified: {:?}", path.as_os_str()),
            )),
        }
    }
}
