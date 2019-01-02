use rocket::{http::Status, response::Stream};
use serde_urlencoded::from_str;
use std::io::Read;
use std::process::Child;

use std::process::Command;

#[derive(Deserialize, Debug)]
struct FmtStreamMap {
    quality: String,
    #[serde(rename = "type")]
    stream_type: String,
    itag: String,
    url: String,
}

#[derive(Deserialize, Debug)]
struct GetVideoInfo {
    url_encoded_fmt_stream_map: String,
}

pub struct ChildKiller(Child);

impl Drop for ChildKiller {
    fn drop(&mut self) {
        self.0.kill();
    }
}

impl Read for ChildKiller {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let stdout = &mut self.0.stdout;
        stdout.as_mut().map_or(
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Stdout was unavailable",
            )),
            |std| std.read(buf),
        )
    }
}

#[get("/audio/<id>")]
pub fn get_audio(id: String) -> Result<Stream<ChildKiller>, Status> {
    let client = reqwest::Client::new();
    let mut res = client
        .get("https://youtube.com/get_video_info")
        .query(&[("video_id", id)])
        .send()
        .map_err(|err| {
            error!("{}", err);
            Status::InternalServerError
        })?;
    if res.status() != reqwest::StatusCode::OK {
        return Err(Status::from_code(res.status().as_u16()).unwrap_or(Status::InternalServerError));
    }
    let body_string = res.text().map_err(|err| {
        error!("{}", err);
        Status::InternalServerError
    })?;
    let video_info: GetVideoInfo = from_str(body_string.as_str()).map_err(|err| {
        error!("{}", err);
        Status::InternalServerError
    })?;

    let mut fmt_stream_map: Vec<FmtStreamMap> = Vec::new();
    for url in video_info.url_encoded_fmt_stream_map.split(',') {
        fmt_stream_map.push(from_str(url).map_err(|err| {
            error!("{}", err);
            Status::InternalServerError
        })?);
    }

    info!("{:?}", fmt_stream_map);

    for fmt_stream in fmt_stream_map {
        if fmt_stream.quality == "hd720" {
            let child = Command::new("ffmpeg")
                .arg("-i")
                .arg(fmt_stream.url)
                .arg("-f")
                .arg("mp3")
                .arg("pipe:1")
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::null())
                .stdin(std::process::Stdio::null())
                .spawn()
                .map_err(|err| {
                    error!("{}", err);
                    Status::InternalServerError
                })?;
            return Ok(Stream::from(ChildKiller(child)));
        }
    }

    Err(Status::NotFound)
}
