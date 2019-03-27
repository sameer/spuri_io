// A special thanks goes to Alexey Golub for his excellent blog post
// on reverse engineering YouTube
// https://tyrrrz.me/Blog/Reverse-engineering-YouTube

use rocket::http::uri::Origin;
use rocket::http::uri::Segments;
use rocket::{http::Status, response::Stream};
use serde_urlencoded::from_str;
use std::io::Read;
use std::process::Child;
use std::process::Command;
use url::Url;

#[derive(Deserialize, Debug)]
struct FmtStream {
    itag: String,
    #[serde(rename = "s")]
    signature: Option<String>,
    quality: String,
    #[serde(rename = "type")]
    stream_type: String,
    sp: Option<String>,
    url: String,
}

#[derive(Deserialize, Debug)]
struct GetVideoInfo {
    url_encoded_fmt_stream_map: String,
    title: String,
    author: String,
}

pub struct ChildKiller(Child);

impl Drop for ChildKiller {
    fn drop(&mut self) {
        self.0.kill().unwrap();
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

#[get("/audio/<segments..>")]
pub fn get_audio(mut segments: Segments, origin: &Origin) -> Result<Stream<ChildKiller>, Status> {
    let id = segments.next().ok_or(Status::InternalServerError)?;
    let id: String = if segments.next().is_some() {
        origin
            .query()
            .map(|x: &str| x[2..].to_string())
            .ok_or(Status::InternalServerError)
    } else {
        Ok(id.to_string())
    }?;
    debug!("{}", id);
    let client = reqwest::Client::new();
    let embed_html = client
        .get(format!("https://www.youtube.com/embed/{}", id).as_str())
        .send()
        .map_err(|err| {
            error!("{}", err);
            Status::InternalServerError
        })?
        .text()
        .map_err(|err| {
            error!("{}", err);
            Status::InternalServerError
        })?;
    let base_js_url = xtract::base_js_url(&embed_html).ok_or(Status::InternalServerError)?;
    debug!("BaseJS URL: {}", base_js_url);
    let base_js = client
        .get(&base_js_url)
        .send()
        .map_err(|err| {
            error!("{}", err);
            Status::InternalServerError
        })?
        .text()
        .map_err(|err| {
            error!("{}", err);
            Status::InternalServerError
        })?;

    let sts = xtract::sts(&base_js).ok_or(Status::InternalServerError)?;
    debug!("STS: {}", sts);

    let mut video_info = client
        .get("https://youtube.com/get_video_info")
        .query(&[
            ("video_id", id),
            ("sts", sts.to_string()),
            ("el", "detailpage".to_string()),
        ])
        .send()
        .map_err(|err| {
            error!("{}", err);
            Status::InternalServerError
        })?;
    if video_info.status() != reqwest::StatusCode::OK {
        return Err(
            Status::from_code(video_info.status().as_u16()).unwrap_or(Status::InternalServerError)
        );
    }
    let video_info: GetVideoInfo = from_str(
        video_info
            .text()
            .map_err(|err| {
                error!("{}", err);
                Status::InternalServerError
            })?
            .as_str(),
    )
    .map_err(|err| {
        error!("{}", err);
        Status::InternalServerError
    })?;
    debug!("{:?}", video_info);

    let mp4_itags = vec!["22", "18", "37", "59", "78"];
    let fmt_streams = video_info
        .url_encoded_fmt_stream_map
        .split(',')
        .try_fold(vec![], |mut acc: Vec<FmtStream>, url_encoded| {
            serde_urlencoded::from_str(url_encoded).map(|fmt_stream| {
                acc.push(fmt_stream);
                acc
            })
        })
        .map_err(|err| {
            error!("{}", err);
            Status::InternalServerError
        })?;
    fmt_streams
        .iter()
        .find(|fmt_stream| mp4_itags.contains(&fmt_stream.itag.as_str()))
        .ok_or(Status::NotFound)
        .and_then(|fmt_stream| {
            Url::parse(&fmt_stream.url)
                .map(|url| (fmt_stream, url))
                .map_err(|err| {
                    error!("{}", err);
                    Status::InternalServerError
                })
        })
        .map(|(fmt_stream, mut fmt_stream_url)| {
            fmt_stream
                .signature
                .clone()
                .and_then(|signature| {
                    xtract::signature_function_name(&base_js)
                        .and_then(|name| xtract::signature_function_calls(&base_js, name))
                        .and_then(|calls| {
                            debug!("Finding function name");
                            xtract::signature_transform_function_name(&calls)
                                .map(|name| (calls, name))
                        })
                        .and_then(|(calls, name)| {
                            debug!("Function name is {}", name);
                            xtract::signature_transform_function_parts(&base_js, &name)
                                .map(|functions| (calls, functions))
                        })
                        .map(|(calls, functions)| {
                            debug!("Applying functions");
                            xtract::apply_signature_transformation(&signature, functions, calls)
                        })
                        .and_then(|new_signature| {
                            let new_query_string: String = fmt_stream_url
                                .query_pairs()
                                .filter(|(query_name, _query_value)| query_name != "signature")
                                .fold(
                                    url::form_urlencoded::Serializer::new(String::new()),
                                    |mut acc, (query_name, query_value)| {
                                        acc.append_pair(
                                            &query_name.to_string(),
                                            &query_value.to_string(),
                                        );
                                        acc
                                    },
                                )
                                .append_pair("signature", &new_signature)
                                .finish();
                            debug!("New query string is {}", new_query_string);
                            fmt_stream_url.set_query(Some(&new_query_string));
                            Some(fmt_stream_url.to_string())
                        })
                })
                .unwrap_or_else(|| fmt_stream_url.to_string())
        })
        .and_then(|fmt_stream_url| {
            debug!("Connecting to URL {}", fmt_stream_url);
            let child = Command::new("ffmpeg")
                .arg("-i")
                .arg(fmt_stream_url)
                .arg("-f")
                .arg("mp3")
                .arg("-metadata")
                .arg(format!("title=\"{}\"", video_info.title))
                .arg("-metadata")
                .arg(format!("author=\"{}\"", video_info.author))
                .arg("pipe:1")
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::null())
                .stdin(std::process::Stdio::null())
                .spawn()
                .map_err(|err| {
                    error!("{}", err);
                    Status::InternalServerError
                })?;
            return Ok(Stream::chunked(ChildKiller(child), 4096));
        })
}

mod xtract {
    use regex::{Regex, RegexBuilder};

    lazy_static! {
        static ref STS_REGEX: Regex = Regex::new(r"sts:(\d+)").unwrap();
        static ref BASE_JS_REGEX: Regex = Regex::new(r"(/yts/jsbin/(?:\w|[-/.])+base\.js)").unwrap();
        static ref SIGNATURE_FUNCTION_NAME_REGEXES: Vec<Regex> = vec![
            // unsupported due to lack of backreferences
            // Regex::new(r#"(["'])signature\1\s*,\s*(?P<sig>[a-zA-Z0-9$]+)\("#).unwrap(),
            Regex::new(r#"yt\.akamaized\.net/\)\s*\|\|\s*.*?\s*c\s*&&\s*d\.set\([^,]+\s*,\s*(?:encodeURIComponent\s*\()?(?P<sig>[a-zA-Z0-9$]+)\("#).unwrap(),
            Regex::new(r#"\bc\s*&&\s*d\.set\([^,]+\s*,\s*(?:encodeURIComponent\s*\()?\s*(?P<sig>[a-zA-Z0-9$]+)\("#).unwrap(),
            Regex::new(r#"\.sig\|\|(?P<sig>[a-zA-Z0-9$]+)\("#).unwrap(),
            Regex::new(r#"yt\.akamaized\.net/\)\s*\|\|\s*.*?\s*c\s*&&\s*d\.set\([^,]+\s*,\s*(?P<sig>[a-zA-Z0-9$]+)\("#).unwrap(),
            Regex::new(r#"\bc\s*&&\s*d\.set\([^,]+\s*,\s*(?P<sig>[a-zA-Z0-9$]+)\("#).unwrap(),
            Regex::new(r#"\bc\s*&&\s*d\.set\([^,]+\s*,\s*\([^)]*\)\s*\(\s*(?P<sig>[a-zA-Z0-9$]+)\("#).unwrap(),
        ];
        static ref SIGNATURE_TRANSFORM_FUNCTION_NAME_REGEX: Regex = Regex::new(r#"(.+?)\."#).unwrap();

        static ref FUNCTION_REGEX: Regex = RegexBuilder::new(r#"(?:([.A-Za-z0-9]+)\s*?:function\((.+?)\)\{(.*?)\}(?:,|\s)*?)"#).multi_line(true).build().unwrap();
        static ref FUNCTION_CALL_REGEX: Regex = Regex::new(r#"([.A-Za-z0-9$_]+)\((.*?)\);"#).unwrap();
    }

    pub fn sts(base_js: &str) -> Option<&str> {
        STS_REGEX
            .captures(base_js)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str())
    }

    pub fn base_js_url(embed_html: &str) -> Option<String> {
        BASE_JS_REGEX
            .captures(embed_html)
            .and_then(|caps| caps.get(1))
            .map(|m| format!("https://www.youtube.com{}", m.as_str()).to_string())
    }

    pub fn signature_function_name(base_js: &str) -> Option<&str> {
        SIGNATURE_FUNCTION_NAME_REGEXES
            .iter()
            .find(|regex| regex.is_match(base_js))
            .and_then(|regex| regex.captures(base_js))
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str())
    }

    pub fn signature_function_calls<'a>(
        base_js: &'a str,
        function_name: &'a str,
    ) -> Option<Vec<JSFunctionCall>> {
        debug!("Finding function calls for {}", function_name);
        Regex::new(
            format!(
                r#"{}=function\(a\)\{{a=a.split\(""\);(.+)return a\.join\(""\)\}};"#,
                function_name
            )
            .as_str(),
        )
        .ok()
        .and_then(|regex| regex.captures(base_js))
        .and_then(|caps| caps.get(1))
        .and_then(|m| {
            debug!("Calls are {}", m.as_str());
            let calls: Vec<JSFunctionCall> = FUNCTION_CALL_REGEX
                .captures_iter(m.as_str())
                .filter_map(|caps| {
                    if let (Some(name), Some(params)) = (caps.get(1), caps.get(2)) {
                        Some(JSFunctionCall {
                            name: name.as_str().to_string(),
                            params: params
                                .as_str()
                                .to_string()
                                .split(',')
                                .map(|x| x.to_string())
                                .collect(),
                        })
                    } else {
                        None
                    }
                })
                .collect();
            if calls.len() == 0 {
                None
            } else {
                Some(calls)
            }
        })
    }

    pub fn signature_transform_function_name(
        signature_function_body: &Vec<JSFunctionCall>,
    ) -> Option<String> {
        signature_function_body.iter().nth(0).and_then(|x| {
            SIGNATURE_TRANSFORM_FUNCTION_NAME_REGEX
                .captures(&x.name)
                .and_then(|caps| caps.get(1))
                .map(|m| m.as_str().to_string())
        })
    }

    #[derive(Debug, PartialEq, Eq, Clone)]
    pub struct JSFunction {
        pub name: Option<String>,
        pub params: Vec<String>,
        pub body: String,
    }

    #[derive(Debug, PartialEq, Eq, Clone)]
    pub struct JSFunctionCall {
        pub name: String,
        pub params: Vec<String>,
    }

    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub enum TransformType {
        Swap,
        Slice,
        Reverse,
        Other,
    }

    impl From<&JSFunction> for TransformType {
        fn from(other: &JSFunction) -> Self {
            if other.body.contains("reverse") {
                Self::Reverse
            } else if other.body.contains("splice") {
                Self::Slice
            } else if other.body.contains('%') && other.body.contains(".length") {
                Self::Swap
            } else {
                Self::Other
            }
        }
    }

    pub fn signature_transform_function_parts<'a>(
        base_js: &'a str,
        function_name: &'a str,
    ) -> Option<Vec<JSFunction>> {
        RegexBuilder::new(
            format!(
                r#"var {}=\{{((?:.|\s)+?)\}};"#,
                regex::escape(function_name)
            )
            .as_str(),
        )
        .multi_line(true)
        .build()
        .ok()
        .and_then(|regex| regex.captures(base_js))
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str())
        .map(|m| {
            debug!("Parts are {}", m);
            m
        })
        .map(|m| FUNCTION_REGEX.captures_iter(m))
        .map(|caps_it| {
            caps_it
                .map(|caps| JSFunction {
                    name: Some(caps.get(1).unwrap().as_str().to_string()),
                    params: caps
                        .get(2)
                        .unwrap()
                        .as_str()
                        .to_string()
                        .split(',')
                        .map(|x| x.to_string())
                        .collect(),
                    body: caps.get(3).unwrap().as_str().to_string(),
                })
                .collect()
        })
    }

    pub fn apply_signature_transformation(
        signature: &str,
        functions: Vec<JSFunction>,
        calls: Vec<JSFunctionCall>,
    ) -> String {
        let mut signature = signature.to_string();
        debug!("Functions are {:#?}, Calls are {:#?}", functions, calls);
        calls.iter().for_each(|call| {
            let call_name = call.name.rsplit('.').next().unwrap();
            functions
                .iter()
                .find(|function| {
                    function
                        .name
                        .clone()
                        .map(|name| name == call_name)
                        .unwrap_or(false)
                })
                .map(|function| {
                    let transform_type: TransformType = function.into();
                    match transform_type {
                        TransformType::Reverse => {
                            signature = signature.chars().rev().collect();
                        }
                        TransformType::Slice => {
                            let i: usize = call.params.iter().last().unwrap().parse().unwrap();
                            signature = signature.split_at(i).1.to_string();
                        }
                        TransformType::Swap => {
                            let i: usize = call.params.iter().last().unwrap().parse().unwrap();
                            let ci: char = signature.chars().nth(i).unwrap();
                            let c0: char = signature.chars().next().unwrap();
                            signature = signature
                                .char_indices()
                                .map(|(j, c)| {
                                    if j == 0 {
                                        ci
                                    } else if j == i {
                                        c0
                                    } else {
                                        c
                                    }
                                })
                                .collect();
                        }
                        TransformType::Other => {}
                    };
                });
        });
        signature
    }

    #[cfg(test)]
    mod test {
        use super::*;

        #[test]
        fn sts_regex_matches() {
            let base_js = include_str!("./tests/base.js");
            let sts = sts(base_js);
            assert!(sts.is_some());
            assert_eq!(sts.unwrap(), "17885");
        }

        #[test]
        fn base_js_regex_matches() {
            let embed_html = include_str!("./tests/tvTRZJ-4EyI");
            let base_js_url = base_js_url(embed_html);
            assert!(base_js_url.is_some());
            assert_eq!(
                base_js_url.unwrap(),
                "https://www.youtube.com/yts/jsbin/player-vflpOZkP0/en_US/base.js"
            );
        }

        #[test]
        fn signature_function_name_regex_matches() {
            let base_js = include_str!("./tests/base.js");
            let signature_function_name = signature_function_name(base_js);
            assert!(signature_function_name.is_some());
            assert_eq!(signature_function_name.unwrap(), "qL");
        }

        #[test]
        fn signature_function_body_regex_matches() {
            let base_js = include_str!("./tests/base.js");
            let function_name = signature_function_name(base_js);
            assert!(function_name.is_some());
            let body = signature_function_calls(base_js, function_name.unwrap());
            assert!(body.is_some());
            assert_eq!(
                body.unwrap(),
                vec![
                    JSFunctionCall {
                        name: "pL.wx".to_string(),
                        params: vec!["a".to_string(), "34".to_string()]
                    },
                    JSFunctionCall {
                        name: "pL.dL".to_string(),
                        params: vec!["a".to_string(), "58".to_string()]
                    },
                    JSFunctionCall {
                        name: "pL.wx".to_string(),
                        params: vec!["a".to_string(), "68".to_string()]
                    },
                    JSFunctionCall {
                        name: "pL.Hy".to_string(),
                        params: vec!["a".to_string(), "1".to_string()]
                    },
                    JSFunctionCall {
                        name: "pL.wx".to_string(),
                        params: vec!["a".to_string(), "38".to_string()]
                    },
                    JSFunctionCall {
                        name: "pL.dL".to_string(),
                        params: vec!["a".to_string(), "46".to_string()]
                    }
                ]
            );
        }

        #[test]
        fn signature_transform_function_name_matches() {
            let base_js = include_str!("./tests/base.js");
            let function_name = signature_function_name(base_js);
            assert!(function_name.is_some());
            let function_body = signature_function_calls(base_js, function_name.unwrap());
            assert!(function_body.is_some());
            let function_body = function_body.unwrap();
            let name = signature_transform_function_name(&function_body);
            assert!(name.is_some());
            assert_eq!(name.unwrap(), "pL");
        }

        #[test]
        fn signature_transform_function_parts_matches() {
            let base_js = include_str!("./tests/base.js");
            let function_name = signature_function_name(base_js);
            assert!(function_name.is_some());
            let function_body = signature_function_calls(base_js, function_name.unwrap());
            assert!(function_body.is_some());
            let function_body = function_body.unwrap();
            let name = signature_transform_function_name(&function_body);
            assert!(name.is_some());
            let name = name.unwrap();
            let body = signature_transform_function_parts(base_js, &name);
            assert!(body.is_some());
            assert_eq!(
                body.unwrap(),
                [
                    JSFunction {
                        name: Some("wx".to_string()),
                        params: vec!["a".to_string(), "b".to_string()],
                        body: "var c=a[0];a[0]=a[b%a.length];a[b%a.length]=c".to_string()
                    },
                    JSFunction {
                        name: Some("dL".to_string()),
                        params: vec!["a".to_string()],
                        body: "a.reverse()".to_string()
                    },
                    JSFunction {
                        name: Some("Hy".to_string()),
                        params: vec!["a".to_string(), "b".to_string()],
                        body: "a.splice(0,b)".to_string()
                    }
                ]
            );
            let body = signature_transform_function_parts(base_js, &name);
            let types: Vec<TransformType> = body.unwrap().iter().map(|x| x.into()).collect();
            assert_eq!(
                types,
                [
                    TransformType::Swap,
                    TransformType::Reverse,
                    TransformType::Slice
                ]
            )
        }

        #[test]
        fn apply_signature_transformation_matches() {
            let base_js = include_str!("./tests/base.js");
            let function_name = signature_function_name(base_js);
            assert!(function_name.is_some());
            let function_body = signature_function_calls(base_js, function_name.unwrap());
            assert!(function_body.is_some());
            let function_body = function_body.unwrap();
            let name = signature_transform_function_name(&function_body);
            assert!(name.is_some());
            let name = name.unwrap();
            let body = signature_transform_function_parts(base_js, &name);
            assert!(body.is_some());
            assert_eq!("68FED286CE1850B6D16F4743AE6D7A3750A361E1.DBE82917965E7925F313F50A4D342313C139D11", 
                apply_signature_transformation("A8FED286CE18E0B6D16F4743AE6D7A37506361E1.1BE82917965E7925F313F50A4D342313C139D1D5", body.unwrap(), function_body));
        }
    }
}
