#[macro_use]
extern crate lazy_static;

use futures::stream::{self, StreamExt};
use regex::{Match, Regex};
use reqwest::Response;
use std::io;
use std::io::BufRead;
use std::sync::Mutex;

lazy_static! {
    static ref BSHORT_REGEX: Regex =
        Regex::new(r"(?P<url>https?://b23.tv/[0-9a-zA-Z]+)\??(?:&?[^=&]*=[^=&]*)*").unwrap();
}

#[tokio::main]
async fn main() {
    // read text from args or stdin
    let mut args = std::env::args();
    let _ = args.next();
    let text = if args.len() > 0 {
        args.reduce(|acc, i| format!("{acc} {i}"))
            .expect("Failed to read args")
    } else {
        let lines = io::stdin().lock().lines();
        lines.fold(String::new(), |mut acc, i| {
            acc.push_str(&*i.unwrap());
            acc
        })
    };
    let string = replace_bshort(&text).await;
    println!("{}", string.into_inner().unwrap())
}

async fn get_redirect_url(url: &str) -> String {
    let resp: Response = reqwest::get(url).await.unwrap();
    let mut x = resp.url().clone();
    x.set_query(None);
    x.to_string()
}

async fn replace_bshort(text: &str) -> Mutex<String> {
    let matches = BSHORT_REGEX.find_iter(text);
    let matches_vec = matches.fold(Vec::new(), |mut acc: Vec<Match>, i| {
        acc.push(i);
        acc
    });
    let mut trim = Mutex::new(String::from(text));
    let mut stream = stream::iter(matches_vec);
    while let Some(x) = stream.next().await {
        let url = x.as_str();
        trim = Mutex::from(text.replace(url, &get_redirect_url(url).await));
    }
    trim
}
