use std::collections::BTreeMap;

extern crate serde;
extern crate serde_json;

use serde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WptReport {
    pub run_info: serde_json::Value,
    pub time_start: u64,
    pub time_end: u64,
    pub results: Vec<TestResult>,
    pub lsan_leaks: Option<Vec<LsanLeak>>,
    pub mozleak: Option<BTreeMap<String, MozLeak>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TestResult {
    pub test: String,
    pub subtests: Vec<SubtestResult>,
    pub status: TestStatus,
    pub expected: Option<TestStatus>,
    pub known_intermittent: Option<Vec<TestStatus>>,
    pub message: Option<String>,
    pub duration: Option<i64>,
    pub asserts: Option<AssertionCount>,
    pub reftest_screenshots: Option<BTreeMap<String, String>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SubtestResult {
    pub name: String,
    pub status: SubtestStatus,
    pub expected: Option<SubtestStatus>,
    pub known_intermittent: Option<Vec<SubtestStatus>>,
    pub message: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AssertionCount {
    pub count: u32,
    pub min: u32,
    pub max: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LsanLeak {
    pub frames: Vec<String>,
    pub scope: String,
    pub allowed_match: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MozLeak {
    pub objects: Vec<MozLeakObject>,
    pub total: Vec<MozLeakTotal>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MozLeakObject {
    pub process: Option<String>,
    pub name: String,
    pub allowed: bool,
    pub bytes: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MozLeakTotal {
    pub bytes: u64,
    pub threshold: u64,
    pub process: Option<String>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug, Copy, Hash)]
pub enum TestStatus {
    PASS,
    FAIL,
    OK,
    ERROR,
    TIMEOUT,
    CRASH,
    ASSERT,
    #[allow(non_camel_case_types)]
    PRECONDITION_FAILED,
    SKIP,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug, Copy, Hash)]
pub enum SubtestStatus {
    PASS,
    FAIL,
    ERROR,
    TIMEOUT,
    ASSERT,
    #[allow(non_camel_case_types)]
    PRECONDITION_FAILED,
    NOTRUN,
    SKIP,
}

// #[cfg(test, Clone, Debug)]
// mod tests {
//     use super::WptReport;
//     use serde_json;
//     use std::fs;
//     use std::io::Read;
//     use std::path::PathBuf;

//     fn get_data_paths() -> Vec<PathBuf> {
//         let mut data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
//         data_dir.push("testdata");
//         println!("{:?}", data_dir);
//         let mut paths = Vec::new();
//         for entry in fs::read_dir(data_dir).unwrap() {
//             let path = entry.unwrap().path();
//             println!("{:?}", path);
//             if path
//                 .file_name()
//                 .unwrap()
//                 .to_str()
//                 .unwrap()
//                 .starts_with("wptreport.json")
//             {
//                 paths.push(path);
//             }
//         }
//         paths
//     }

//     #[test]
//     fn parse_examples() {
//         for path in get_data_paths() {
//             println!("{:?}", path);
//             let mut buf = String::new();
//             let mut f = fs::File::open(path).unwrap();
//             f.read_to_string(&mut buf).unwrap();
//             let _: WptReport = serde_json::from_str(&buf).unwrap();
//         }
//     }
// }
