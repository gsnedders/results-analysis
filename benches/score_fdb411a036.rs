use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::time::Duration;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use flate2::read::GzDecoder;
use wpt_data::bsf::{score_runs, score_runs_total};
use wpt_data::wptreport::WptReport;

pub fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("score fdb411a036");
    group.measurement_time(Duration::new(20, 0));

    let mut data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    data_dir.push("testdata");

    let mut data: Vec<WptReport> = vec![];

    for name in [
        "fdb411a036-chrome-99.0.4818.0_dev-linux-20.04-20e5be768d.json.gz",
        "fdb411a036-firefox-98.0a1-linux-20.04-e6197f6b17.json.gz",
        "fdb411a036-safari-137_preview-mac-10.16-4e9a953e75.json.gz",
    ] {
        let mut path = data_dir.clone();
        path.push(name);
        let mut f = GzDecoder::new(fs::File::open(path).unwrap());
        let mut buf = String::new();
        f.read_to_string(&mut buf).unwrap();
        data.push(serde_json::from_str(&buf).unwrap());
    }

    group.bench_with_input(BenchmarkId::new("score fdb411a036", 1), &data, |b, s| {
        b.iter(|| score_runs_total(score_runs(s)).expect("error!"));
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
