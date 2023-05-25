use futures::future::try_join_all;
use {reqwest, tokio};

use crate::wptreport::WptReport;

static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

//const CONCURRENT_REQUESTS: usize = 6;

use serde_derive::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Run {
    pub id: i64,
    pub browser_name: String,
    pub browser_version: String,
    pub os_name: String,
    pub os_version: String,
    pub revision: String,
    pub full_revision_hash: String,
    pub results_url: String,
    pub created_at: String,
    pub time_start: String,
    pub time_end: String,
    pub raw_results_url: String,
    pub labels: Vec<String>,
}

pub fn fetch_runs(url: &str) -> Result<Vec<(Run, WptReport)>, reqwest::Error> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let client = reqwest::Client::builder()
        .user_agent(APP_USER_AGENT)
        .gzip(true)
        .build()?;

    let runs: Vec<Run> = rt.block_on(async { client.get(url).send().await?.json().await })?;

    let raw_results: Vec<WptReport> = rt.block_on(async {
        try_join_all(runs.iter().map(|run| &run.raw_results_url).map(|url| {
            let client = &client;
            async move { client.get(url).send().await?.json::<WptReport>().await }
        }))
        .await
    })?;

    Ok(runs.into_iter().zip(raw_results.into_iter()).collect())
}
