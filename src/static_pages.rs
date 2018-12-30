use askama::Template;
use base::*;
use rocket::State;

#[derive(Template)]
#[template(path = "about.html")]
pub struct About {
    _parent: Base,
}

#[get("/about")]
pub fn get_about(state: State<Base>) -> About {
    About {
        _parent: state.inner().clone(),
    }
}

#[derive(Template)]
#[template(path = "index.html")]
pub struct Index {
    _parent: Base,
}

#[get("/")]
pub fn get_index(req: State<Base>) -> Index {
    Index {
        _parent: req.inner().clone(),
    }
}
