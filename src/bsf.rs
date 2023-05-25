use std::cmp::max;
use std::collections::hash_map::Entry;
use std::collections::HashMap;

use accurate::sum::OnlineExactSum;
use accurate::traits::SumAccumulator;
use conv::ValueInto;
use rayon::iter::plumbing::UnindexedConsumer;
use rayon::iter::ParallelBridge;
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use smallvec::SmallVec;

use crate::utils::{AlignedSubtests, AlignedTests};
use crate::wptreport::{SubtestResult, SubtestStatus, TestResult, TestStatus, WptReport};

fn score_subtest_mut(run_results: &[SmallVec<[&SubtestResult; 1]>], scores: &mut [usize]) {
    #[allow(clippy::enum_glob_use)]
    use SubtestStatus::*;

    assert!(run_results.len() > 1);
    assert_eq!(run_results.len(), scores.len());

    let mut failing_result = None;
    for (i, results) in run_results.iter().enumerate() {
        let status = match results[..] {
            [] => None,
            [r, ..] => Some(r.status),
        };

        match status {
            Some(SKIP | PRECONDITION_FAILED) => return,
            None | Some(FAIL | ERROR | TIMEOUT | NOTRUN) => match failing_result {
                None => failing_result = Some(i),
                Some(_) => return,
            },
            Some(PASS | ASSERT) => (),
        }
    }

    match failing_result {
        None => (),
        Some(i) => scores[i] += 1,
    }
}

fn score_subtests_mut<'a, 'b, 'c>(
    test_results: &[&'a Vec<SubtestResult>],
    scores: &'b mut Vec<f64>,
) -> Result<(), &'c str> {
    assert!(test_results.len() > 1);
    assert!(scores.is_empty());

    let mut subtest_total = [0].repeat(test_results.len());

    let aligned = AlignedSubtests::new(test_results.iter().map(|x| x.iter()));
    let mut results = 0;

    for (i, (_, subtest)) in aligned.into_iter().enumerate() {
        score_subtest_mut(&subtest, &mut subtest_total);
        results = i + 1;
    }

    assert!(results > 0);
    let subtest_count: f64 = results.value_into().map_err(|_| "too many subtests")?;

    for subtest_score in &subtest_total {
        let subtest_score_f: f64 = (*subtest_score)
            .value_into()
            .map_err(|_| "too many subtests")?;
        #[allow(clippy::float_arithmetic)]
        scores.push(subtest_score_f / subtest_count);
    }

    assert_eq!(scores.len(), test_results.len());

    Ok(())
}

fn score_test_no_subtest_mut(run_results: &[SmallVec<[&TestResult; 1]>], scores: &mut Vec<f64>) {
    #[allow(clippy::enum_glob_use)]
    use TestStatus::*;

    assert!(run_results.len() > 1);
    assert!(scores.is_empty());

    scores.resize(run_results.len(), 0.into());

    let mut failing_result = None;
    for (i, results) in run_results.iter().enumerate() {
        let result = results.get(0);

        assert!(match result {
            Some(r) => r.subtests.is_empty(),
            None => true,
        });

        match result.map(|r| r.status) {
            None | Some(SKIP | PRECONDITION_FAILED) => return,
            Some(FAIL | ERROR | TIMEOUT | CRASH) => match failing_result {
                None => failing_result = Some(i),
                Some(_) => return,
            },
            Some(OK | PASS | ASSERT) => (),
        }
    }

    match failing_result {
        None => (),
        Some(i) => scores[i] = 1_u8.into(),
    }

    assert_eq!(scores.len(), run_results.len());
}

pub fn score_test_mut<'a, 'b, 'c>(
    run_results: &'a [SmallVec<[&TestResult; 1]>],
    scores: &'b mut Vec<f64>,
) -> Result<(), &'c str> {
    assert!(run_results.len() > 1);
    assert!(scores.is_empty());

    let have_multiple_results = run_results.iter().any(|r| r.len() > 1);
    let has_subtests = run_results
        .iter()
        .filter(|rs| !rs.get(0).map_or(true, |r| r.subtests.is_empty()))
        .count();

    if have_multiple_results {
        return Err("have multiple results for a single test");
    } else if has_subtests == run_results.len() {
        let subtests: Vec<_> = run_results.iter().flatten().map(|x| &x.subtests).collect();
        score_subtests_mut(&subtests, scores)?;
    } else if has_subtests > 0 {
        scores.resize(run_results.len(), 0.into());
    } else {
        score_test_no_subtest_mut(run_results, scores);
    }

    assert_eq!(scores.len(), run_results.len());
    //println!("{}", format!("{},{:?},{:?},{:?}", run_results[0][0].test, scores[0], scores[1], scores[2]));

    Ok(())
}

pub fn score_test<'a, 'b>(
    run_results: &'a [SmallVec<[&TestResult; 1]>],
) -> Result<Vec<f64>, &'b str> {
    let mut score: Vec<f64> = Vec::with_capacity(run_results.len());
    score_test_mut(run_results, &mut score)?;
    Ok(score)
}

#[derive(Debug, Clone)]
#[must_use]
#[allow(clippy::type_complexity)]
pub struct TestScoreIterator<'a>(
    rayon::iter::Map<
        rayon::iter::IterBridge<AlignedTests<'a>>,
        fn(<AlignedTests<'a> as Iterator>::Item) -> <Self as ParallelIterator>::Item,
    >,
);

impl<'a> ParallelIterator for TestScoreIterator<'a> {
    type Item = (&'a str, Result<Vec<f64>, &'static str>);
    fn drive_unindexed<C>(self, c: C) -> C::Result
    where
        C: UnindexedConsumer<Self::Item>,
    {
        self.0.drive_unindexed(c)
    }
}

pub fn score_runs(runs: &[WptReport]) -> TestScoreIterator {
    let aligned = AlignedTests::new(runs.iter().map(|x| &x.results));
    // Using ParallelBridge here is fine, because the the iterator is relatively quick, whereas the map is slow.
    TestScoreIterator(
        aligned
            .into_iter()
            .par_bridge()
            .map(|(k, v)| (k, score_test(&v))),
    )
}

pub fn score_runs_total<'a, I>(tests_scores: I) -> Result<Vec<f64>, &'static str>
where
    I: IntoParallelIterator<Item = <TestScoreIterator<'a> as ParallelIterator>::Item>,
{
    tests_scores
        .into_par_iter()
        .map(|(_, test_scores)| test_scores)
        .try_fold(
            Vec::new,
            |mut runs_totals: Vec<OnlineExactSum<f64>>, scores: Result<Vec<f64>, _>| {
                let scores = scores?;
                if runs_totals.is_empty() {
                    runs_totals.resize_with(scores.len(), OnlineExactSum::zero);
                }
                for (run_total, score) in runs_totals.iter_mut().zip(scores.into_iter()) {
                    #[allow(clippy::float_arithmetic)]
                    (*run_total += score);
                }
                Ok(runs_totals)
            },
        )
        .filter(|x| x.as_ref().map_or(true, |y| !y.is_empty()))
        .try_reduce_with(|runs_totals, scores| {
            Ok(runs_totals
                .into_iter()
                .zip(scores.into_iter())
                .map(|(run_total, score)| run_total + score)
                .collect::<Vec<_>>())
        })
        .unwrap_or(Err("no test scores found?"))
        .map(|scores| {
            scores
                .into_iter()
                .map(OnlineExactSum::sum)
                .collect::<Vec<_>>()
        })
}

pub fn score_runs_by_dir<'a, I>(
    tests_scores: I,
    recursive: bool,
) -> Result<HashMap<String, Vec<f64>>, &'static str>
where
    I: IntoParallelIterator<Item = <TestScoreIterator<'a> as ParallelIterator>::Item>,
{
    tests_scores
        .into_par_iter()
        .try_fold(
            HashMap::new,
            |mut dir_scores: HashMap<String, Vec<OnlineExactSum<f64>>>,
             (test, scores): (&str, Result<Vec<f64>, _>)| {
                let scores = scores?;
                let path = test
                    .split_once(&['?', '#'])
                    .map_or(test, |(prefix, _)| prefix);
                for i in path
                    .rmatch_indices('/')
                    .take(if recursive { usize::MAX } else { 1 })
                    .map(|(i, _)| i)
                    .map(|i| max(i, 1))
                {
                    let value = dir_scores
                        .entry(test[..i].to_string())
                        .or_insert_with(|| scores.iter().map(|_| OnlineExactSum::zero()).collect());
                    for (total_score, score) in value.iter_mut().zip(scores.iter()) {
                        #[allow(clippy::float_arithmetic)]
                        (*total_score += *score);
                    }
                }

                Ok(dir_scores)
            },
        )
        .try_reduce(HashMap::new, |mut a, b| {
            for (dir, scores) in b {
                match a.entry(dir) {
                    Entry::Occupied(mut o) => {
                        o.insert(
                            o.get()
                                .into_iter()
                                .zip(scores.into_iter())
                                .map(|(run_total, score)| run_total.clone() + score)
                                .collect::<Vec<_>>(),
                        );
                    }
                    Entry::Vacant(v) => {
                        v.insert(scores);
                    }
                }
            }
            Ok(a)
        })
        .map(|dir_scores| {
            dir_scores
                .into_iter()
                .map(|(k, v)| (k, v.into_iter().map(OnlineExactSum::sum).collect()))
                .collect()
        })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Read;
    use std::path::PathBuf;

    use flate2::read::GzDecoder;
    use serde_json;
    use smallvec::smallvec;

    use super::{WptReport, *};

    fn score_subtest_new(results: &[Option<&SubtestResult>]) -> Vec<usize> {
        let r_vec: Vec<_> = results
            .iter()
            .map(|r| match r {
                None => smallvec![],
                Some(s) => smallvec![*s],
            })
            .collect();

        let mut subtest_total = [0].repeat(results.len());
        score_subtest_mut(&r_vec, &mut subtest_total);
        subtest_total
    }

    #[test]
    fn test_score_subtest() {
        let result_fail = SubtestResult {
            name: "Test".to_string(),
            status: SubtestStatus::FAIL,
            expected: None,
            known_intermittent: None,
            message: None,
        };
        let result_pass = SubtestResult {
            name: "Test".to_string(),
            status: SubtestStatus::PASS,
            expected: None,
            known_intermittent: None,
            message: None,
        };

        assert_eq!(
            vec![0, 0],
            score_subtest_new(&[Some(&result_pass), Some(&result_pass)])
        );
        assert_eq!(
            vec![0, 0],
            score_subtest_new(&[Some(&result_fail), Some(&result_fail)])
        );
        assert_eq!(
            vec![1, 0],
            score_subtest_new(&[Some(&result_fail), Some(&result_pass)])
        );
        assert_eq!(
            vec![0, 1],
            score_subtest_new(&[Some(&result_pass), Some(&result_fail)])
        );
    }

    fn score_subtests_new(results: &[&Vec<SubtestResult>]) -> Vec<f64> {
        let mut total = vec![];
        score_subtests_mut(results, &mut total).expect("Scoring must succeed");
        total
    }

    #[test]
    fn test_score_subtests() {
        let result1_fail = SubtestResult {
            name: "Test1".to_string(),
            status: SubtestStatus::FAIL,
            expected: None,
            known_intermittent: None,
            message: None,
        };
        let result1_pass = SubtestResult {
            name: "Test1".to_string(),
            status: SubtestStatus::PASS,
            expected: None,
            known_intermittent: None,
            message: None,
        };
        let _result2_fail = SubtestResult {
            name: "Test2".to_string(),
            status: SubtestStatus::FAIL,
            expected: None,
            known_intermittent: None,
            message: None,
        };
        let result2_pass = SubtestResult {
            name: "Test2".to_string(),
            status: SubtestStatus::PASS,
            expected: None,
            known_intermittent: None,
            message: None,
        };

        let subtest_results_pass = vec![result1_pass, result2_pass.clone()];
        let subtest_results_fail = vec![result1_fail, result2_pass];

        let run_subtests: Vec<_> = vec![&subtest_results_fail, &subtest_results_pass];

        let scored = score_subtests_new(&run_subtests);

        assert_eq!(vec![0.5, 0.0], scored,);
    }

    // fn get_data_paths() -> Vec<PathBuf> {
    //     let mut data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    //     data_dir.push("testdata");
    //     println!("{:?}", data_dir);
    //     let mut paths = Vec::new();
    //     for entry in fs::read_dir(data_dir).unwrap() {
    //         let path = entry.unwrap().path();
    //         println!("{:?}", path);
    //         if path
    //             .file_name()
    //             .unwrap()
    //             .to_str()
    //             .unwrap()
    //             .starts_with("wptreport.json")
    //         {
    //             paths.push(path);
    //         }
    //     }
    //     paths
    // }

    #[test]
    fn score_examples() {
        let mut data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        data_dir.push("testdata");

        let data: Vec<WptReport> = [
            "fdb411a036-chrome-99.0.4818.0_dev-linux-20.04-20e5be768d.json.gz",
            "fdb411a036-firefox-98.0a1-linux-20.04-e6197f6b17.json.gz",
            "fdb411a036-safari-137_preview-mac-10.16-4e9a953e75.json.gz",
        ]
        .into_par_iter()
        .map(|name| {
            let mut path = data_dir.clone();
            path.push(name);
            let mut f = GzDecoder::new(fs::File::open(path).unwrap());
            let mut buf = String::new();
            f.read_to_string(&mut buf).unwrap();
            serde_json::from_str(&buf).unwrap()
        })
        .collect();

        let scores = score_runs_total(score_runs(&data)).expect("error!");
        assert_eq!(
            scores,
            vec![590.0798357956824, 1576.770933380237, 3416.5829135383365]
        );
    }
}
