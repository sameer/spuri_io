use askama::Template;
use lazy_static::lazy_static;

lazy_static! {
    static ref CSS_FILE_HASH: String = {
        let css_file_contents = include_str!("../files/style.css");
        let mut hasher = openssl::sha::Sha512::new();
        hasher.update(css_file_contents.as_bytes());
        let hash: &[u8] = &hasher.finish();
        format!("sha512-{}", base64::encode(hash))
    };
    pub static ref BASE: Base = Base {
        css_file_hash: &CSS_FILE_HASH,
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
}

#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub struct NavItem {
    pub link: &'static str,
    pub name: &'static str,
    pub new_page: bool,
}

#[derive(Template, Clone, Hash, Eq, PartialEq, Debug)]
#[template(path = "base.html")]
pub struct Base {
    pub nav_items: [NavItem; 4],
    pub css_file_hash: &'static str,
}
