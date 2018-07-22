use askama::Template;

pub const BASE: Base = Base {
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

#[derive(Clone)]
pub struct NavItem {
    pub link: &'static str,
    pub name: &'static str,
    pub new_page: bool,
}

#[derive(Template, Clone)]
#[template(path = "base.html")]
pub struct Base {
    pub nav_items: [NavItem; 4],
    pub css_file_hash: &'static str,
}
