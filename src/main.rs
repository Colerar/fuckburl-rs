use std::fs;
use std::io::BufRead;
use std::ops::Sub;
use std::path::{Path, PathBuf};
use std::{io, sync::LazyLock};

use anyhow::Context;
use futures::stream::{self, StreamExt};
use image::{DynamicImage, Rgba};
use qrcode::QrCode;
use regex::Regex;
use reqwest::Response;

static BSHORT_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?P<url>https?://b23.tv/[0-9a-zA-Z]+)\??(?:&?[^=&]*=[^=&]*)*").unwrap()
});

/// Input types, Text or Image
#[derive(Debug)]
enum InputType {
    Text { text: String, is_from_args: bool },
    Image { file: PathBuf },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // read text from args or stdin
    let mut args = std::env::args();
    let _ = args.next();
    let text = if args.len() > 0 {
        let str = args
            .reduce(|acc, i| format!("{acc} {i}"))
            .expect("Failed to read args");
        if fs::metadata(&str).is_ok() {
            InputType::Image {
                file: PathBuf::from(str),
            }
        } else {
            InputType::Text {
                text: str,
                is_from_args: true,
            }
        }
    } else {
        let lines = io::stdin().lock().lines();
        let text = lines.fold(String::new(), |mut acc, i| {
            acc.push_str(&i.unwrap());
            acc
        });
        InputType::Text {
            text,
            is_from_args: false,
        }
    };
    match text {
        InputType::Text { text, is_from_args } => {
            print!(
                "{}{}",
                replace_bshort(&text).await,
                if is_from_args { "\n" } else { "" }
            )
        }
        InputType::Image { file } => {
            let img = replace_qrcode(&file).await?;
            img.save(file)?;
        }
    }

    Ok(())
}

async fn get_redirect_url(url: &str) -> String {
    let resp: Response = reqwest::get(url).await.unwrap();
    let mut x = resp.url().clone();
    x.set_query(None);
    x.to_string()
}

async fn replace_qrcode(file: &Path) -> anyhow::Result<DynamicImage> {
    let origin_img = image::open(file).context("Failed to open image")?;
    // Prepare for detection
    let mut rqrr_detect = rqrr::PreparedImage::prepare(origin_img.to_luma8());
    let mut new_img = origin_img;
    let grids = rqrr_detect.detect_grids();
    for grid in grids {
        let (top_left, top_right, _bottom_right, bottom_left) = (
            grid.bounds[0],
            grid.bounds[1],
            grid.bounds[2],
            grid.bounds[3],
        );
        let width = (top_right.x - top_left.x) as i64;
        let height = (bottom_left.y - top_left.y) as i64;
        let (_meta, content) = grid.decode()?;
        let replaced = replace_bshort(&content).await;
        if replaced == content {
            break;
        }

        // encode the content to qrcode, and put it on the image
        let qr = QrCode::with_error_correction_level(&replaced, qrcode::EcLevel::L)
            .context("Failed to encode to qrcode")?;
        let qrimg = qr
            .render::<Rgba<u8>>()
            .light_color(Rgba([255, 255, 255, 255]))
            .dark_color(Rgba([0, 0, 0, 255]))
            .quiet_zone(false)
            .max_dimensions((width as f64 * 1.1) as u32, (height as f64 * 1.1) as u32)
            .min_dimensions((width as f64 * 0.9) as u32, (height as f64 * 0.9) as u32)
            .build();

        image::imageops::overlay(
            &mut new_img,
            &qrimg,
            top_left.x.sub(5).into(),
            top_left.y.sub(5).into(),
        );
    }
    Ok(new_img)
}

async fn replace_bshort(text: &str) -> String {
    let matches: Vec<_> = BSHORT_REGEX.find_iter(text).collect();
    let mut trim = String::from(text);
    let mut stream = stream::iter(matches);
    while let Some(x) = stream.next().await {
        let url = x.as_str();
        trim = text.replace(url, &get_redirect_url(url).await);
    }
    trim
}
