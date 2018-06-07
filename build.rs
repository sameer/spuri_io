extern crate askama;
extern crate openssl;
extern crate base64;

use std::fs::File;
use std::io::prelude::*;

fn main() {
    askama::rerun_if_templates_changed();
    let hash = match File::open("static/style.css") {
        Ok(mut f) => {
            let mut hasher = openssl::sha::Sha512::new();
            let mut buf = vec![];
            match f.read_to_end(&mut buf) {
                Ok(_) => {
                    hasher.update(&buf);
                    let hash: &[u8] = &hasher.finish();
                    base64::encode(hash)
                }
                Err(_) => "".to_string(),
            }
        }
        Err(_) => "".to_string(),
    };
    let hash = format!("sha512-{}", hash);
    let mut f = File::create("src/css_file_hash").unwrap();
    f.write_all(hash.as_bytes()).unwrap();
    f.sync_all().unwrap();
}
