//! The use-case folder format, parsed once and used by every runner.
//!
//! A case folder is a **use case**, not a test for one binary: `case.txt` names
//! the intent, `details.txt` says who wants it and why, `given.md` is the document
//! before and `expect.md` the document after. Those four are the same whatever
//! interface a person uses.
//!
//! Each interface then declares **how a person does it there**, in a driver file —
//! `cli.txt` for a command line, `tui.txt` for a screen. The driver is the test
//! entry point: a case runs on an interface only when it has one, which is how
//! `core/` holds a case today that the CLI cannot do yet. The format itself is
//! documented where the cases live, in `use-cases/README.md`.

use std::error::Error;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

pub type CaseResult = Result<(), Box<dyn Error>>;

/// One case folder, opened in its own scratch directory.
pub struct Case {
    /// The scratch directory, deleted when the case ends.
    pub scratch: tempfile::TempDir,
    /// `TODOs.md` in the scratch directory.
    pub input: PathBuf,
    /// What `TODOs.md` held before the case ran, for the "must not write" check.
    pub before: Option<Vec<u8>>,
    /// `expect.md`, when the case expects a write.
    pub expect: Option<String>,
    /// `stdout.md`, when the case expects exact output.
    pub stdout: Option<String>,
    /// `details.txt`: what this use case is for. Never asserted — it is the part a
    /// person reads.
    pub details: Option<String>,
}

impl Case {
    /// Open the case a driver file belongs to.
    pub fn open(driver: &Path) -> Result<Self, Box<dyn Error>> {
        let folder = driver.parent().ok_or("a driver file without a folder")?;
        let scratch = tempfile::tempdir()?;
        copy_fixtures(folder, scratch.path())?;
        let input = scratch.path().join("TODOs.md");
        // `bulk.txt` writes that many todos instead of a `given.md`: a case about a
        // limit must reach it without a 200 KB fixture in the repository
        if let Some(count) = read_optional(&folder.join("bulk.txt"))? {
            let count: usize = count.trim().parse()?;
            let mut text = String::from("# too many tasks\n\n");
            for index in 1..=count {
                writeln!(text, "- [ ] (A) bulk task {index}")?;
            }
            fs::write(&input, text)?;
        }
        Ok(Self {
            before: fs::read(&input).ok(),
            expect: read_optional(&folder.join("expect.md"))?,
            stdout: read_optional(&folder.join("stdout.md"))?,
            details: read_optional(&folder.join("details.txt"))?,
            input,
            scratch,
        })
    }

    /// `TODOs.md` must be exactly `expect.md`, or untouched when there is none.
    pub fn check_file(&self) -> CaseResult {
        let actual = fs::read(&self.input).ok();
        let (wanted, what) = match &self.expect {
            Some(expected) => (Some(expected.as_bytes().to_vec()), "TODOs.md against expect.md"),
            None => (self.before.clone(), "TODOs.md unchanged"),
        };
        expect(&actual, &wanted, what)
    }

    pub fn target(&self) -> &Path {
        &self.input
    }

    pub fn substitute(&self, argument: &str) -> String {
        argument
            .replace("{input}", &self.input.display().to_string())
            .replace("{dir}", &self.scratch.path().display().to_string())
    }
}

/// `cli.txt`: the argv a command-line user would type, then what the run must
/// print and return.
///
/// ```
/// import {dir}/tasks.json --into {input}
/// exit: 0
/// stdout: 1 added
/// ```
#[derive(Debug, Default)]
pub struct Cli {
    pub argv: Vec<String>,
    pub exit: i32,
    pub stdout_contains: Vec<String>,
    pub stderr_contains: Vec<String>,
}

impl Cli {
    pub fn read(driver: &Path) -> Result<Self, Box<dyn Error>> {
        let mut cli = Self::default();
        for line in driver_lines(&fs::read_to_string(driver)?) {
            match line.split_once(':') {
                Some((key, value)) => match key.trim() {
                    "exit" => cli.exit = value.trim().parse()?,
                    "stdout" => cli.stdout_contains.push(value.trim().to_owned()),
                    "stderr" => cli.stderr_contains.push(value.trim().to_owned()),
                    // an argv line that happens to hold a colon
                    _ => cli.argv.push(line),
                },
                None => cli.argv.push(line),
            }
        }
        if cli.argv.is_empty() {
            return Err(format!("{}: no command line", driver.display()).into());
        }
        Ok(cli)
    }

    /// stdout: every wanted substring, and nothing at all when none is named.
    pub fn check_stdout(&self, stdout: &str, exact: Option<&str>) -> CaseResult {
        for wanted in &self.stdout_contains {
            expect_contains(stdout, wanted, "stdout")?;
        }
        match exact {
            Some(expected) => expect(
                &stdout.trim_end(),
                &expected.trim_end(),
                "stdout against stdout.md",
            ),
            None if self.stdout_contains.is_empty() => {
                expect(&stdout.trim_end(), &"", "empty stdout")
            }
            None => Ok(()),
        }
    }

    pub fn check_stderr(&self, stderr: &str) -> CaseResult {
        for wanted in &self.stderr_contains {
            expect_contains(stderr, wanted, "stderr")?;
        }
        Ok(())
    }
}

/// `tui.txt`: what a screen user presses, and what must be drawn.
///
/// ```
/// open: file
/// key: x
/// screen: [x] (A) pay rent
/// ```
///
/// `open:` is the only directive that is not a step: `file` opens `TODOs.md`
/// itself (single-file mode), `dir` — the default — opens the folder it sits in.
pub struct Tui {
    /// `key:`, `text:` and `wait:`, in the order they happen.
    pub steps: Vec<String>,
    /// `screen:` lines: text the finished screen must show.
    pub screens: Vec<String>,
    /// Whether the case opens the file rather than its folder.
    pub open_file: bool,
}

impl Tui {
    pub fn read(driver: &Path) -> Result<Self, Box<dyn Error>> {
        let mut steps = Vec::new();
        let mut screens = Vec::new();
        let mut open_file = false;
        for line in driver_lines(&fs::read_to_string(driver)?) {
            if let Some(value) = line.strip_prefix("screen:") {
                screens.push(value.trim().to_owned());
            } else if let Some(value) = line.strip_prefix("open:") {
                open_file = value.trim() == "file";
            } else {
                steps.push(line);
            }
        }
        if !ends_with_wait(&steps) {
            return Err(format!(
                "{}: the steps must end with a `wait:` line — it is what proves the \
                 last action landed before the file is compared",
                driver.display()
            )
            .into());
        }
        Ok(Self {
            steps,
            screens,
            open_file,
        })
    }
}

fn ends_with_wait(steps: &[String]) -> bool {
    steps.last().is_some_and(|line| line.starts_with("wait:"))
}

/// The lines of a driver file: leading whitespace and comments dropped, trailing
/// whitespace kept — a `text:` step may need to type a space.
pub fn driver_lines(text: &str) -> Vec<String> {
    text.lines()
        .map(|line| line.trim_start().trim_end_matches('\r'))
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_owned)
        .collect()
}

fn copy_fixtures(folder: &Path, scratch: &Path) -> Result<(), Box<dyn Error>> {
    for item in fs::read_dir(folder)?.flatten() {
        let name = item.file_name().to_string_lossy().into_owned();
        let target = if name == "given.md" {
            Some(PathBuf::from("TODOs.md"))
        } else {
            name.strip_prefix("fixture-").map(PathBuf::from)
        };
        if let Some(target) = target {
            if let Some(parent) = scratch.join(&target).parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(item.path(), scratch.join(target))?;
        }
    }
    Ok(())
}

pub fn read_optional(path: &Path) -> Result<Option<String>, Box<dyn Error>> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

pub fn expect<T: PartialEq + std::fmt::Debug>(actual: &T, wanted: &T, what: &str) -> CaseResult {
    if actual == wanted {
        Ok(())
    } else {
        Err(format!("{what}\n  wanted: {wanted:?}\n  actual: {actual:?}").into())
    }
}

pub fn expect_contains(haystack: &str, needle: &str, what: &str) -> CaseResult {
    if haystack.contains(needle) {
        Ok(())
    } else {
        Err(format!("{what} lacks {needle:?}\n  actual: {haystack}").into())
    }
}

/// Poll `check` until it passes or the deadline runs out. The one wait in the
/// harness, so no case sleeps for a fixed time.
pub fn eventually(
    mut check: impl FnMut() -> CaseResult,
    timeout: std::time::Duration,
) -> CaseResult {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        match check() {
            Ok(()) => return Ok(()),
            Err(error) => {
                if std::time::Instant::now() >= deadline {
                    return Err(error);
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
    }
}
