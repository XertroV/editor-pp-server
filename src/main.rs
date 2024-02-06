#[macro_use]
extern crate simple_log;


// use image::{io::Reader as ImageReader, ImageFormat};
// use simple_log::log_level;
use tokio::{io::AsyncWriteExt, task, stream::{self}};
use tokio_stream::{StreamExt};
// use webp::{Encoder, WebPMemory};
// use image_convert::{ImageResource, PNGConfig, to_png};
use std::{error::Error, ffi::OsString, io::{BufReader, Cursor, Read, Seek}, net::IpAddr, path::{Path, PathBuf}, str::FromStr};
use std::net::SocketAddr;
// use crate::config::*;

use bytes::Bytes;
use warp::{reply::{Reply, WithStatus}, Filter};
use base64::prelude::*;
use text_to_ascii_art::convert;

mod config;

#[tokio::main]
async fn main() {
    // set log level to info
    println!("{}\n\nQuestions and support: @XertroV on Openplanet discord.\n\n", convert("E++ Server".into()).unwrap());
    simple_log::quick!("info");
    match run_app().await {
        Ok(_) => (),
        Err(e) => error!("Error: {:?}", e)
    };
    info!("Server shutting down...");
    std::thread::sleep(std::time::Duration::from_secs(1));
}

async fn run_app() -> Result<(), Box<dyn std::error::Error>> {
    let config = crate::config::load_config();
    let ip_addr = IpAddr::from_str(&config.server.host)?;
    let soc_addr: SocketAddr = SocketAddr::new(ip_addr, config.server.port);


    info!("Enabling route: POST e++/lm-analysis/convert/webp");
    let lm_analysis = warp::path!("e++" / "lm-analysis" / "convert" / "webp")
        .and(warp::path::end())
        .and(warp::body::bytes())
        .and_then(|b| async move {handle_analysis_raw_conversion(b).await})
        ;

    let no_local = config.server.no_local.unwrap_or(false);
    if no_local {
        warn!("Local mode is disabled");
    } else {
        info!("Enabling route: POST e++/lm-analysis/convert/webp/local");
    }

    let lm_analysis_local_route = warp::post()
        .and(warp::path!("e++" / "lm-analysis" / "convert" / "webp" / "local"))
        .and(warp::path::end())
        .and(warp::body::bytes())
        .and_then(move |b| async move {
            if !no_local {
                handle_analysis_local_conversion(b).await
            } else {
                Ok(warp::reply::with_status("Local conversion is disabled".into(), warp::http::StatusCode::FORBIDDEN))
            }
        })
        ;

    warp::serve(lm_analysis.or(lm_analysis_local_route).with(warp::log("lm-analysis")))
        .run(soc_addr)
        .await;

    info!("Server shutting down.");

    Ok(())
}


async fn handle_analysis_raw_conversion(body_base64: Bytes) -> Result<impl warp::Reply, warp::Rejection> {
    // body is base64 encoded zip file
    let body = match BASE64_STANDARD.decode(body_base64) {
        Ok(b) => b,
        Err(e) => return Ok(warp::reply::with_status(format!("Error: {:?}", e).into(), warp::http::StatusCode::BAD_REQUEST))
    };
    let reader = std::io::BufReader::new(Cursor::new(body));
    reader_to_result(reader).await
}


async fn handle_analysis_local_conversion(body: Bytes) -> Result<WithStatus<Vec<u8>>, warp::Rejection> {
    let path_str: String = body.to_vec().iter().map(|b| *b as char).collect();
    let path = PathBuf::from(path_str.clone());
    // check it exists
    if !Path::new(&path).exists() {
        warn!("Could not find file: {}", path_str);
        return Ok(warp::reply::with_status(format!("File not found {:?}", path.as_os_str()).into(), warp::http::StatusCode::NOT_FOUND));
    }
    if path.extension() != Some(OsString::from("zip").as_os_str()) {
        return Ok(warp::reply::with_status("File is not a zip".into(), warp::http::StatusCode::BAD_REQUEST));
    }
    // unzip
    let file = std::fs::File::open(path).unwrap();
    let reader = std::io::BufReader::new(file);
    reader_to_result(reader).await
}


async fn reader_to_result<R>(reader: BufReader<R>) -> Result<WithStatus<Vec<u8>>, warp::Rejection>
    where R: Read + Sized + Seek
{
    info!("Processing zip file...");

    let output_imgs = match process_zip(reader).await {
        Ok(output_imgs) => output_imgs,
        Err(e) => return Ok(warp::reply::with_status(format!("Error: {:?}", e).into(), warp::http::StatusCode::INTERNAL_SERVER_ERROR))
    };

    info!("Converted {} images: {}", output_imgs.len(), output_imgs.iter().map(|(name, _)| name.clone()).collect::<Vec<String>>().join(", "));

    let res = prep_output_buf(output_imgs).await;

    match res {
        Ok(buf) => Ok(warp::reply::with_status(buf, warp::http::StatusCode::OK)),
        Err(e) => Ok(warp::reply::with_status(format!("Error: {:?}", e).into(), warp::http::StatusCode::INTERNAL_SERVER_ERROR))
    }
}


async fn process_zip<R>(reader: BufReader<R>) -> Result<Vec<(String, Cursor<Vec<u8>>)>, Box<dyn Error>>
    where R: Read + Sized + Seek
    {
    let mut zip = zip::ZipArchive::new(reader).unwrap();
    let mut output_imgs: Vec<(String, Cursor<Vec<u8>>)> = vec![];
    let mut archives = vec![];
    for i in 0..zip.len() {
        let mut file = zip.by_index(i).unwrap();
        let path = match file.enclosed_name() {
            Some(path) => path.to_owned(),
            None => return Err("bad path".into()),
        };
        info!("Processing file: {}", path.to_str().unwrap());
        let mut data = Vec::new();
        file.read_to_end(&mut data).unwrap();
        archives.push((path, data));
    }

    let futures: Vec<_> = archives.into_iter().map(|(path, data)| {
        task::spawn_blocking(move || {
            if path.to_str() == Some("ProbeGrid.webp") || (path.to_str().unwrap().starts_with("LightMap") && path.to_str().unwrap().ends_with(".webp")) {
                info!("[Thread {:?}] Loading image: {}", std::thread::current().id(), path.to_str().unwrap());
                let im = image::load_from_memory(&data).unwrap();
                info!("[Thread {:?}] Flipping image: {}", std::thread::current().id(), path.to_str().unwrap());
                let im = im.flipv();
                let mut bs: Cursor<Vec<u8>> = Cursor::new(Vec::new());
                // let mut writer = std::io::BufWriter::new(&mut bs);
                info!("[Thread {:?}] Saving image: {}", std::thread::current().id(), path.to_str().unwrap());
                im.write_to(&mut bs, image::ImageOutputFormat::Png).unwrap();
                let result = (path.to_str().unwrap().replace(".webp", ".png"), bs);
                info!("[Thread {:?}] Done image: {}\n", std::thread::current().id(), path.to_str().unwrap());
                return Ok(result);
            } else {
                info!("[Thread {:?}] Skipping file: {}\n", std::thread::current().id(), path.to_str().unwrap_or("could not convert path to file name"));
                return Err("skipped");
            }
        })
    }).collect();

    for f in futures {
        let res = f.await.unwrap();
        match res {
            Ok(r) => output_imgs.push(r),
            Err(_) => ()
        };
    }

    Ok(output_imgs)
}

async fn prep_output_buf(output_imgs: Vec<(String, Cursor<Vec<u8>>)>) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut output_buf: Vec<u8> = Vec::new();
    for (name, mut img) in output_imgs {
        output_buf.write_u32_le(name.len() as u32).await?;
        output_buf.extend_from_slice(format!("{}", name).as_bytes());
        img.set_position(0);
        output_buf.write_u32_le(img.get_ref().len() as u32).await?;
        output_buf.extend_from_slice(img.get_ref());
    }
    Ok(output_buf)
}

/*
def process_lm_file(name: str, zf: zipfile.ZipFile) -> BytesIO | None:
    can_process = name == "ProbeGrid.webp" or (name.startswith("LightMap") and name.endswith(".webp"))
    if not can_process: return None
    data = zf.read(name)
    im = Image.open(BytesIO(data), formats=['webp'])
    im = im.transpose(Image.Transpose.FLIP_TOP_BOTTOM)
    bs = BytesIO()
    im.save(bs, "png")
    return bs

*/
