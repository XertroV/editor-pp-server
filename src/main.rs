#[macro_use]
extern crate simple_log;


use image::{io::Reader as ImageReader, ImageFormat};
use simple_log::log_level;
use tokio::io::AsyncWriteExt;
// use webp::{Encoder, WebPMemory};
// use image_convert::{ImageResource, PNGConfig, to_png};
use std::{error::Error, ffi::OsString, io::{Cursor, Read}, net::{IpAddr, Ipv4Addr}, path::{Path, PathBuf}, str::FromStr};
use std::net::SocketAddr;
// use crate::config::*;

use bytes::Bytes;
use warp::Filter;

mod config;

#[tokio::main]
async fn main() {
    // set log level to info
    simple_log::quick!("info");
    match run_app().await {
        Ok(_) => (),
        Err(e) => eprintln!("Error: {:?}", e)
    };
}

async fn run_app() -> Result<(), Box<dyn std::error::Error>> {
    let config = crate::config::load_config();
    let ip_addr = IpAddr::from_str(&config.server.host)?;
    let soc_addr: SocketAddr = SocketAddr::new(ip_addr, config.server.port);

    // POST e++/lm-analysis/convert/webp
    // let lm_analysis = warp::path!("e++" / "lm-analysis" / "convert" / "webp")
    //     .map();


    let lm_analysis_route = warp::post()
        .and(warp::path!("e++" / "lm-analysis" / "convert" / "webp" / "local"))
        .and(warp::body::bytes())
        .and_then(|b| async move {handle_analysis_conversion(b).await});


    // // GET /hello/warp => 200 OK with body "Hello, warp!"
    // let hello = warp::path!("hello" / String)
    //     .map(|name| format!("Hello, {}!", name));

    warp::serve(lm_analysis_route)
        .run(soc_addr)
        .await;

    Ok(())
}


async fn handle_analysis_conversion(body: Bytes) -> Result<impl warp::Reply, warp::Rejection> {
    let path_str: String = body.to_vec().iter().map(|b| *b as char).collect();
    let path = PathBuf::from(path_str);
    // check it exists
    if !Path::new(&path).exists() {
        return Ok(warp::reply::with_status("File not found".into(), warp::http::StatusCode::NOT_FOUND));
    }
    if path.extension() != Some(OsString::from("zip").as_os_str()) {
        return Ok(warp::reply::with_status("File is not a zip".into(), warp::http::StatusCode::BAD_REQUEST));
    }
    let output_imgs = match process_zip(path).await {
        Ok(output_imgs) => output_imgs,
        Err(e) => return Ok(warp::reply::with_status(format!("Error: {:?}", e).into(), warp::http::StatusCode::INTERNAL_SERVER_ERROR))
    };

    let res = prep_output_buf(output_imgs).await;
    match res {
        Ok(buf) => Ok(warp::reply::with_status(buf, warp::http::StatusCode::OK)),
        Err(e) => Ok(warp::reply::with_status(format!("Error: {:?}", e).into(), warp::http::StatusCode::INTERNAL_SERVER_ERROR))
    }
}

async fn process_zip(path: PathBuf) -> Result<Vec<(String, Cursor<Vec<u8>>)>, Box<dyn Error>> {
    // unzip
    let file = std::fs::File::open(path).unwrap();
    let reader = std::io::BufReader::new(file);

    let mut zip = zip::ZipArchive::new(reader).unwrap();
    let mut output_imgs: Vec<(String, Cursor<Vec<u8>>)> = vec![];
    for i in 0..zip.len() {
        let mut file = zip.by_index(i).unwrap();
        let path = match file.enclosed_name() {
            Some(path) => path.to_owned(),
            None => continue,
        };

        if path.to_str() == Some("ProbeGrid.webp") || (path.to_str().unwrap().starts_with("LightMap") && path.to_str().unwrap().ends_with(".webp")) {
            let mut data = Vec::new();
            file.read_to_end(&mut data).unwrap();
            let im = image::load_from_memory(&data).unwrap();
            let im = im.flipv();
            let mut bs: Cursor<Vec<u8>> = Cursor::new(Vec::new());
            // let mut writer = std::io::BufWriter::new(&mut bs);
            im.write_to(&mut bs, image::ImageOutputFormat::Png).unwrap();
            output_imgs.push((path.to_str().unwrap().replace(".webp", ".png"), bs));
        }
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



fn convert_webp_to_png(input_path: &str, output_path: &str) -> Result<(), image::ImageError> {
    // Load the WebP image
    let img = ImageReader::open(input_path)?
        .with_guessed_format()?
        .decode()?;

    // Save the image as PNG
    img.save_with_format(output_path, ImageFormat::Png)?;

    Ok(())
}
