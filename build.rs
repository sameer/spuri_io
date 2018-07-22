extern crate askama;
extern crate base64;
extern crate openssl;

use std::fs::File;
use std::io::prelude::*;

fn main() {
    askama::rerun_if_templates_changed();

    let mut css_file = File::open("files/style.css").unwrap();
    let mut hasher = openssl::sha::Sha512::new();
    let mut css_file_contents = vec![];
    css_file.read_to_end(&mut css_file_contents).unwrap();
    hasher.update(&css_file_contents);
    let hash: &[u8] = &hasher.finish();
    let hash = format!("sha512-{}", base64::encode(hash));
    let mut css_file_hash = File::create("src/css_file_hash").unwrap();
    css_file_hash.write_all(hash.as_bytes()).unwrap();
    css_file_hash.sync_all().unwrap();
}
