use askama::Template;

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
