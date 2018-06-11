extern crate askama;
extern crate base64;
extern crate openssl;

use std::fs::File;
use std::io::prelude::*;

fn main() {
    askama::rerun_if_templates_changed();
    let mut f = File::open("files/style.css").unwrap();
    let mut hasher = openssl::sha::Sha512::new();
    let mut buf = vec![];
    f.read_to_end(&mut buf).unwrap();
    hasher.update(&buf);
    let hash: &[u8] = &hasher.finish();
    let hash = format!("sha512-{}", base64::encode(hash));
    let mut f = File::create("src/css_file_hash").unwrap();
    f.write_all(hash.as_bytes()).unwrap();
    f.sync_all().unwrap();
}
