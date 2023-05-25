use std::iter::{Fuse, Peekable};
use std::vec;

use peeking_take_while::PeekableExt;
use smallvec::SmallVec;

use crate::wptreport::{SubtestResult, TestResult};

pub trait TestName {
    fn id(&self) -> &str;
}

impl TestName for TestResult {
    fn id(&self) -> &str {
        &self.test
    }
}

impl TestName for SubtestResult {
    fn id(&self) -> &str {
        &self.name
    }
}

#[derive(Debug, Clone)]
#[must_use]
pub struct AlignedResults<'a, T: TestName> {
    sorted: Vec<Peekable<Fuse<vec::IntoIter<&'a T>>>>,
    curr: Option<&'a str>,
}

impl<'a, T: TestName> AlignedResults<'a, T> {
    fn sort_key<'x>(r: &&'x T) -> (usize, &'x str) {
        // Sort by length first, as comparing integers is quicker than comparing strings, and we
        // merely care about providing a consistent sort where each test name is grouped.
        (r.id().len(), r.id())
    }

    pub fn new<X, Y>(test_results: X) -> AlignedResults<'a, T>
    where
        X: IntoIterator<Item = Y>,
        Y: IntoIterator<Item = &'a T>,
    {
        let browser_tests = test_results
            .into_iter()
            .map(|browser| {
                let mut tests: Vec<&T> = browser.into_iter().collect();
                tests.sort_by_key(AlignedResults::sort_key);
                tests.into_iter().fuse().peekable()
            })
            .collect::<Vec<_>>();

        AlignedResults {
            sorted: browser_tests,
            curr: None,
        }
    }
}

impl<'a, T: TestName> Iterator for AlignedResults<'a, T> {
    type Item = (&'a str, SmallVec<[SmallVec<[&'a T; 1]>; 5]>);
    fn next(&mut self) -> Option<Self::Item> {
        self.curr = self
            .sorted
            .iter_mut()
            .flat_map(|x| x.peek().map(AlignedResults::sort_key).into_iter())
            .min()
            .map(|(_, x)| x);

        let curr = self.curr?;
        let item = self
            .sorted
            .iter_mut()
            .map(|x| {
                x.by_ref()
                    .into_iter()
                    .peeking_take_while(|y| y.id() == curr)
                    .collect::<SmallVec<_>>()
            })
            .collect::<SmallVec<_>>();

        Some((curr, item))
    }
}

pub type AlignedTests<'a> = AlignedResults<'a, TestResult>;
pub type AlignedSubtests<'a> = AlignedResults<'a, SubtestResult>;
