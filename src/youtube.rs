use rocket::{http::Status, response::Stream};
use serde_urlencoded::from_str;

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

#[get("/audio/<id>")]
pub fn get_audio(id: String) -> Result<Stream<reqwest::Response>, Status> {
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
            Command::new("ffmpeg")
                .arg("-i")
                .arg(fmt_stream.url)
                .arg("-listen")
                .arg("1")
                .arg("http://localhost:8080/out.mp3")
                .spawn()
                .map_err(|err| {
                    error!("{}", err);
                    Status::InternalServerError
                })?;
            std::thread::sleep(std::time::Duration::from_secs(1));
            let res = client
                .get("http://localhost:8080/out.mp3")
                .send()
                .map_err(|err| {
                    error!("{}", err);
                    Status::InternalServerError
                })?;
            return Ok(Stream::from(res));
        }
    }

    Err(Status::NotFound)
}
