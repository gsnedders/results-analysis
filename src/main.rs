#![warn(clippy::enum_glob_use)]
#![warn(clippy::float_arithmetic)]
#![warn(clippy::from_iter_instead_of_collect)]
#![warn(clippy::unnested_or_patterns)]

pub mod bsf;
pub mod fetch_wptfyi;
pub mod utils;
pub mod wptreport;

use std::collections::BTreeMap;
use std::io;

use clap::{Parser, Subcommand};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use url::Url;

use crate::bsf::{score_runs, score_runs_by_dir};
use crate::fetch_wptfyi::{fetch_runs, Run};
use crate::wptreport::WptReport;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(long)]
    label: Vec<String>,

    #[arg(long)]
    product: Vec<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Bsf { sha: String },
    BsfTests { sha: String },
}

fn main() {
    let cli = Cli::parse();
    match &cli.command {
        Commands::Bsf { sha } | Commands::BsfTests { sha } => {
            let labels = cli.label;
            let product = cli.product;

            let runs = Url::parse_with_params(
                "https://wpt.fyi/api/runs?aligned=true&max-count=1",
                labels
                    .into_iter()
                    .map(|x| ("label", x))
                    .chain(product.iter().map(|x| ("product", x.to_string())))
                    .chain([("sha", sha.to_string())]),
            );

            let run_reports: Vec<(Run, WptReport)> =
                fetch_runs(runs.unwrap().as_ref()).expect("couldn't fetch!");
            // let runs: Vec<&Run> = run_reports.iter().map(|&(ref run, _)| run).collect();
            let reports: Vec<WptReport> = run_reports
                .iter()
                .map(|&(_, ref report)| report.clone())
                .collect();

            let test_scores: BTreeMap<&str, Result<Vec<f64>, _>> =
                score_runs(&reports).into_par_iter().collect();

            let mut wtr = csv::Writer::from_writer(io::stdout());

            match &cli.command {
                Commands::Bsf { .. } => {
                    wtr.write_record(
                        ["dir".to_string()]
                            .into_iter()
                            .chain(product.iter().map(|x| x.to_string() + "_tot"))
                            .chain(product.iter().map(|x| x.to_string() + "_self")),
                    )
                    .unwrap();

                    let mut self_dir_scores: BTreeMap<_, _> =
                        score_runs_by_dir(test_scores.clone(), false)
                            .unwrap()
                            .into_iter()
                            .collect();

                    let tot_dir_scores: BTreeMap<_, _> =
                        score_runs_by_dir(test_scores.clone(), true)
                            .unwrap()
                            .into_iter()
                            .collect();

                    let default: Vec<f64> = [0_f64].repeat(product.len());

                    for (dir, scores) in tot_dir_scores.into_iter() {
                        let self_scores = self_dir_scores
                            .remove(&dir)
                            .unwrap_or_else(|| default.clone());
                        wtr.write_record(
                            [dir.to_string()]
                                .into_iter()
                                .chain(scores.into_iter().map(|x| x.to_string()))
                                .chain(self_scores.into_iter().map(|x| x.to_string())),
                        )
                        .unwrap();
                    }

                    assert_eq!(self_dir_scores.len(), 0);
                }
                Commands::BsfTests { .. } => {
                    wtr.write_record(["test".to_string()].into_iter().chain(product.into_iter()))
                        .unwrap();

                    for (test, scores) in test_scores.into_iter() {
                        wtr.write_record(
                            [test.to_string()]
                                .into_iter()
                                .chain(scores.unwrap().into_iter().map(|x| x.to_string())),
                        )
                        .unwrap();
                    }
                }
            }
        }
    }
}
