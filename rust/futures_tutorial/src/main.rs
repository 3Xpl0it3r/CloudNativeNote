use color_eyre::Report;
use std::future::Future;
use reqwest::Client;
use tracing::info;

mod dumb;


pub const URL_1: &str = "https://fasterthanli.me/articles/whats-in-the-box";
pub const URL_2: &str = "https://fasterthanli.me/series/advent-of-code-2020/part-13";

#[tokio::main]
async fn main() -> Result<(), Report>  {
    setup()?;

    let client = Client::new();
    let fut = fetch_thing(&client, URL_1);
    fut.await?;
    Ok(())
}


async fn fetch_thing(client: &Client, url: &str) -> Result<(), Report> {
    let ret = client.get(url).send().await?.error_for_status()?;
    info!(%url, content_type=?ret.headers().get("content-type"), "got a response");
    Ok(())
}


fn fetch_thing_crunchy<'a>(client: &'a Client, url: &'a str) -> impl Future<Output = Result<(), Report>> + 'a {
    async move {
        let res = client.get(url).send().await?.error_for_status()?;
        info!(%url, content_type = ?res.headers().get("conent-type"), "Got    a response");
        Ok(())
    }
}


fn setup() -> Result<(), Report> {
    if std::env::var("RUST_LIB_BACKTRACE").is_err() {
        std::env::set_var("RUST_LIB_BACKTRACE", "0");
    }
    color_eyre::install()?;
    if std::env::var("RUST_LOG").is_err(){
        std::env::set_var("RUST_LOG", "info");
    }
    tracing_subscriber::fmt::fmt().init();
    Ok(())
}
