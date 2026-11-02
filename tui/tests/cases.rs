//! Every `use-cases/tui/**/case.txt` is one test, driven through a real pty.
//!
//! The TUI is only observable the way a user observes it — keys in, screen out,
//! file on disk afterwards. `vt100` turns the pty's bytes back into a screen so
//! `wait:` can poll for text instead of sleeping. See `use-cases/README.md`.

use std::error::Error;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use testkit::cases::{driver_lines, eventually, expect, Case, CaseResult};
use vt100::Parser;

// `harness!` generates `main`, so it belongs at module level.
datatest_stable::harness! {
    { test = run, root = concat!(env!("CARGO_MANIFEST_DIR"), "/../use-cases/tui"), pattern = r"^.*case\.txt$" },
}

const ROWS: u16 = 24;
const COLS: u16 = 80;

fn rows() -> std::num::NonZeroU16 {
    std::num::NonZeroU16::new(ROWS).unwrap_or(std::num::NonZeroU16::MIN)
}

fn cols() -> std::num::NonZeroU16 {
    std::num::NonZeroU16::new(COLS).unwrap_or(std::num::NonZeroU16::MIN)
}
const WAIT: Duration = Duration::from_secs(5);

/// One case folder, driven through the TUI binary under a pty.
fn run(case_file: &Path) -> CaseResult {
    let folder = case_file.parent().ok_or("case.txt without a folder")?;
    let case = Case::open(case_file)?;
    let steps = driver_lines(&fs::read_to_string(folder.join("keys.txt"))?);

    // `single.txt` asks for the file, not the folder it sits in.
    let opened = if folder.join("single.txt").exists() {
        case.target().to_path_buf()
    } else {
        case.scratch.path().to_path_buf()
    };
    let mut terminal = Terminal::spawn(&opened)?;
    let outcome = play(&mut terminal, &steps, &case);
    terminal.stop();
    outcome
}

/// A case must end on a `wait:`, because the screen can only show the state the
/// app reaches *after* it has written. Without one, a case whose expected file
/// equals its input would pass on the untouched file, before the key landed.
fn play(terminal: &mut Terminal, steps: &[String], case: &Case) -> CaseResult {
    if !ends_with_wait(steps) {
        return Err("keys.txt must end with a `wait:` line: it proves the last \
                    action was applied before the file is compared"
            .into());
    }
    for line in steps {
        let Some((verb, value)) = line.split_once(':') else {
            return Err(format!("expected `verb: value`, got {line:?}").into());
        };
        match verb.trim() {
            // one space after the colon is the separator; the rest is typed as is
            "wait" => terminal.wait_for(value.trim(), case)?,
            "key" => terminal.send(&key(value.trim())?)?,
            "text" => terminal.send(value.strip_prefix(' ').unwrap_or(value).as_bytes())?,
            other => return Err(format!("unknown step: {other}").into()),
        }
    }
    terminal.settle();
    let expected = case.output.clone();
    let input = case.target().to_path_buf();
    eventually(
        || match &expected {
            Some(text) => expect(
                &fs::read_to_string(&input).ok(),
                &Some(text.clone()),
                &format!("TODOs.md\n  screen:\n{}", terminal.screen()),
            ),
            None => expect(
                &fs::read_to_string(&input).ok().map(String::into_bytes),
                &case.before.clone(),
                &format!("TODOs.md unchanged\n  screen:\n{}", terminal.screen()),
            ),
        },
        WAIT,
    )
}

fn ends_with_wait(steps: &[String]) -> bool {
    steps.last().is_some_and(|line| line.starts_with("wait:"))
}

/// The bytes a named key sends. Anything one character long is itself.
fn key(name: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let bytes = match name {
        "tab" => "\t",
        "backtab" => "\u{1b}[Z",
        "up" => "\u{1b}[A",
        "down" => "\u{1b}[B",
        "right" => "\u{1b}[C",
        "left" => "\u{1b}[D",
        "home" => "\u{1b}[H",
        "end" => "\u{1b}[F",
        "delete" => "\u{1b}[3~",
        "space" => " ",
        "enter" => "\r",
        "esc" => "\u{1b}",
        "ctrl-c" => "\u{3}",
        single => {
            if single.chars().count() == 1 {
                single
            } else {
                return Err(format!("unknown key: {single}").into());
            }
        }
    };
    Ok(bytes.as_bytes().to_vec())
}

/// A live pty, its child, and the screen the child has drawn so far.
struct Terminal {
    writer: Box<dyn Write + Send>,
    screen: Arc<Mutex<Parser>>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
}

impl Terminal {
    fn spawn(directory: &Path) -> Result<Self, Box<dyn Error>> {
        let pair = native_pty_system().openpty(PtySize {
            rows: ROWS,
            cols: COLS,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        let mut command = CommandBuilder::new(env!("CARGO_BIN_EXE_aricma-lazytodo"));
        command.arg(directory);
        command.env("TERM", "xterm-256color");
        let child = pair.slave.spawn_command(command)?;
        drop(pair.slave);

        let mut reader = pair.master.try_clone_reader()?;
        let writer = pair.master.take_writer()?;
        let screen = Arc::new(Mutex::new(Parser::new(rows(), cols(), 0)));
        let shared = Arc::clone(&screen);
        std::thread::spawn(move || {
            let mut buffer = [0_u8; 4096];
            while let Ok(read) = reader.read(&mut buffer) {
                if read == 0 {
                    break;
                }
                if let Ok(mut parser) = shared.lock() {
                    parser.process(&buffer[..read]);
                }
            }
        });
        Ok(Self {
            writer,
            screen,
            child,
        })
    }

    fn send(&mut self, bytes: &[u8]) -> Result<(), Box<dyn Error>> {
        self.writer.write_all(bytes)?;
        self.writer.flush()?;
        Ok(())
    }

    /// The auto-wait: poll the emulated screen until the text is on it.
    fn wait_for(&self, text: &str, case: &Case) -> CaseResult {
        let _ = case;
        eventually(
            || {
                if self.screen().contains(text) {
                    Ok(())
                } else {
                    Err(format!("waiting for {text:?} on screen:\n{}", self.screen()).into())
                }
            },
            WAIT,
        )
    }

    /// Wait until the screen stops changing, so a write that lands after the
    /// last redraw cannot be missed.
    fn settle(&self) {
        let mut last = String::new();
        for _ in 0..20 {
            let now = self.screen();
            if now == last {
                return;
            }
            last = now;
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    fn screen(&self) -> String {
        self.screen
            .lock()
            .map(|parser| parser.screen().contents())
            .unwrap_or_default()
    }

    fn stop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
