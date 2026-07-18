use std::fs;
use std::path::{Path, PathBuf};

pub struct Case(PathBuf);

impl Case {
    pub fn name(&self) -> &str {
        self.0.file_name().unwrap().to_str().unwrap()
    }

    pub fn read(&self, file: &str) -> String {
        fs::read_to_string(self.0.join(file)).unwrap()
    }
}

pub fn cases(suite: &str) -> Vec<Case> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/cases")
        .join(suite);
    let mut cases = fs::read_dir(root)
        .unwrap()
        .map(|entry| Case(entry.unwrap().path()))
        .filter(|case| case.0.is_dir())
        .collect::<Vec<_>>();
    assert!(!cases.is_empty(), "fixture suite `{suite}` is empty");
    cases.sort_by(|left, right| left.0.cmp(&right.0));
    cases
}

pub fn assert_contains(case: &Case, actual: &str) {
    for expected in case
        .read("to-be.txt")
        .lines()
        .filter(|line| !line.is_empty())
    {
        assert!(
            actual.contains(expected),
            "{}: missing {expected:?}\n\n{actual}",
            case.name()
        );
    }
}
