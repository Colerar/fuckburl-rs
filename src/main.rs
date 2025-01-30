use std::io::BufRead;
use std::{io, sync::LazyLock};

use futures::stream::{self, StreamExt};
use regex::Regex;
use reqwest::Response;

static BSHORT_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?P<url>https?://b23.tv/[0-9a-zA-Z]+)\??(?:&?[^=&]*=[^=&]*)*").unwrap()
});

#[tokio::main]
async fn main() {
    // read text from args or stdin
    let mut args = std::env::args();
    let _ = args.next();
    let (text, is_from_args) = if args.len() > 0 {
        let text = args
            .reduce(|acc, i| format!("{acc} {i}"))
            .expect("Failed to read args");
        (text, true)
    } else {
        let lines = io::stdin().lock().lines();
        let text = lines.fold(String::new(), |mut acc, i| {
            acc.push_str(&i.unwrap());
            acc
        });
        (text, false)
    };
    print!(
        "{}{}",
        replace_bshort(&text).await,
        if is_from_args { "\n" } else { "" }
    )
}

async fn get_redirect_url(url: &str) -> String {
    let resp: Response = reqwest::get(url).await.unwrap();
    let mut x = resp.url().clone();
    x.set_query(None);
    x.to_string()
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
