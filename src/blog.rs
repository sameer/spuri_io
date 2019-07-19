use askama::Template;
use base::*;
use chrono::offset::Utc;
use chrono::DateTime;
use err;
use pulldown_cmark::{Options, Parser};
use rocket::{http::Status, State};
use std::collections::HashMap;
use std::env;
use std::error;
use std::fs;
use std::io;
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

const INDEX_MAX_SIZE: usize = 10;

#[derive(Template, Clone)]
#[template(path = "blog_index.html")]
pub struct Blog {
    _parent: Arc<Base>,
    index: Vec<Post>,
    posts_by_title: HashMap<String, Post>,
}

const FOLDER_PATH: &str = "./blog";

#[get("/")]
pub fn get_index(state: State<BlogState>) -> Blog {
    state.inner().read().unwrap().clone()
}

#[get("/<post>")]
pub fn get_post(state: State<BlogState>, post: String) -> Result<Post, Status> {
    state
        .read()
        .unwrap()
        .posts_by_title
        .get(&post)
        .map(|post| post.clone())
        .ok_or(Status::NotFound)
}

type BlogState = RwLock<Blog>;
impl Blog {
    fn initialize(&mut self) {
        match env::current_dir().and_then(|cwd_path_buf| {
            let gallery_prefix = cwd_path_buf.join(PathBuf::from(FOLDER_PATH));
            fs::read_dir(gallery_prefix)
        }) {
            Ok(dir_iter) => {
                let length_before = self.posts_by_title.len();
                let path_iter = dir_iter
                    .map(|dir_entry_result| dir_entry_result.map(|dir_entry| dir_entry.path()));
                path_iter.for_each(|path_result| {
                    match path_result {
                        Ok(path) => match Post::try_from((self._parent.clone(), &path)) {
                            Ok(post) => {
                                debug!("Adding {}", post.title);
                                self.posts_by_title.insert(post.title.clone(), post.clone());
                                self.index.push(post.clone());
                            }
                            Err(err) => warn!("Couldn't derive new post by path: {}", err),
                        },
                        Err(err) => {
                            warn!("Error while reading file from directory: {}", err);
                        }
                    };
                });
                let length_after = self.posts_by_title.len();
                info!("Found {} posts", length_after - length_before);
            }
            Err(err) => {
                error!("Error while reading files from directory: {}", err);
            }
        };
    }

    pub fn new(parent: Arc<Base>) -> BlogState {
        let mut blog = Blog::from(parent);
        blog.initialize();
        RwLock::new(blog)
    }
}

impl From<Arc<Base>> for Blog {
    fn from(parent: Arc<Base>) -> Blog {
        Blog {
            _parent: parent,
            index: Vec::new(),
            posts_by_title: HashMap::new(),
        }
    }
}

#[derive(Template, Hash, Eq, PartialEq, Debug, Clone)]
#[template(path = "blog_page.html", escape = "none")]
pub struct Post {
    _parent: Arc<Base>,
    title: String,
    last_modified: String,
    author: String,
    checksum: String,
    body: String,
}

impl Post {
    fn path_to_title(path: &PathBuf) -> Option<String> {
        path.file_stem()
            .and_then(|stem_os_str| stem_os_str.to_str())
            .and_then(|stem_str| Some(stem_str.to_string()))
    }

    fn path_to_last_modified(path: &PathBuf) -> Result<String, io::Error> {
        fs::metadata(&path)
            .and_then(|metadata| metadata.modified())
            .map(|systime| {
                let datetime: DateTime<Utc> = systime.into();
                datetime.to_rfc2822()
            })
    }

    fn path_to_author(path: &PathBuf) -> Option<String> {
        Some(String::new())
    }

    fn body_to_checksum(body: &String) -> String {
        let hash: &[u8] = &openssl::sha::sha512(body.as_bytes());
        format!("sha512-{}", base64::encode(hash))
    }

    fn path_to_body(path: &PathBuf) -> Result<String, io::Error> {
        let mut markdown_buf = String::new();
        fs::File::open(&path)
            .and_then(|mut file| file.read_to_string(&mut markdown_buf).map(|_| markdown_buf))
            .map(|markdown_text| {
                let mut opts = Options::empty();
                opts.insert(Options::ENABLE_FOOTNOTES);
                let parser = Parser::new_ext(&markdown_text, opts);
                let mut unsafe_html_text = String::new();
                pulldown_cmark::html::push_html(&mut unsafe_html_text, parser);
                ammonia::Builder::default()
                    .add_tags(&["video"])
                    .add_tag_attributes("video", &["controls", "src"])
                    .add_tag_attributes("div", &["id"])
                    .add_tag_attribute_values("div", "class", &["footnote-definition"])
                    .add_tag_attribute_values("sup", "class", &["footnote-definition-label"])
                    .clean(&*unsafe_html_text)
                    .to_string()
            })
    }

    fn try_from<'a>((base, path): (Arc<Base>, &'a PathBuf)) -> Result<Self, Box<error::Error>> {
        match (
            Post::path_to_title(&path),
            Post::path_to_last_modified(&path),
            Post::path_to_author(&path),
            Post::path_to_body(&path),
        ) {
            (Some(title), Ok(last_modified), Some(author), Ok(body)) => Ok(Post {
                _parent: base,
                title,
                last_modified,
                author,
                checksum: Post::body_to_checksum(&body),
                body,
            }),
            (None, _, _, _) => Err(err::unicode_error(&path)),
            (_, Err(err), _, _) => Err(Box::new(err)),
            (_, _, None, _) => Err(err::unicode_error(&path)),
            (_, _, _, Err(err)) => Err(Box::new(err)),
        }
    }
}
