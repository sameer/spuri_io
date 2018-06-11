use askama::Template;

pub struct NavItem<'a> {
    pub link: &'a str,
    pub name: &'a str,
    pub new_page: bool,
}

impl<'a> Clone for NavItem<'a> {
    fn clone(&self) -> NavItem<'a> {
        NavItem {
            link: self.link,
            name: self.name,
            new_page: self.new_page,
        }
    }
}

#[derive(Template)]
#[template(path = "base.html")]
pub struct Base<'a> {
    pub nav_items: [NavItem<'a>; 4],
    pub css_file_hash: &'a str,
}

impl<'a> Clone for Base<'a> {
    fn clone(&self) -> Base<'a> {
        Base {
            nav_items: self.nav_items.clone(),
            css_file_hash: self.css_file_hash,
        }
    }
}
