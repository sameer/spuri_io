const ROBOTS_TXT: &str = "User-agent: *
Disallow: /files";

#[get("/robots.txt")]
pub fn get_robots_txt() -> &'static str {
    ROBOTS_TXT
}
