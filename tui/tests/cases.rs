//! Every `tui.txt` under `use-cases/` is one test, driven through a real pty.
//!
//! The TUI is only observable the way a user observes it — keys in, screen out,
//! file on disk afterwards. `vt100` turns the pty's bytes back into a screen so
//! `wait:` can poll for text instead of sleeping, and `screen:` says what the
//! finished screen must show. See `use-cases/README.md`.

use std::error::Error;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use testkit::cases::{eventually, expect, expect_contains, Case, CaseResult, Tui};
use vt100::Parser;

// `harness!` generates `main`, so it belongs at module level.
// one root, because a case may live in `core/` or in `tui/`: the pattern is what
// decides which interface it runs on, so a folder with no `tui.txt` is simply not
// collected
datatest_stable::harness! {
    { test = run, root = concat!(env!("CARGO_MANIFEST_DIR"), "/../use-cases"), pattern = r"tui\.txt$" },
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
fn run(driver: &Path) -> CaseResult {
    let case = Case::open(driver)?;
    let tui = Tui::read(driver)?;
    let opened = if tui.open_file {
        case.target().to_path_buf()
    } else {
        case.scratch.path().to_path_buf()
    };
    let mut terminal = Terminal::spawn(&opened)?;
    let outcome = play(&mut terminal, &tui, &case);
    terminal.stop();
    outcome
}

/// The steps, then the file, then the screen: a case proves what a person would
/// check — the document they were editing, and the screen they were looking at.
fn play(terminal: &mut Terminal, tui: &Tui, case: &Case) -> CaseResult {
    for line in &tui.steps {
        let Some((verb, value)) = line.split_once(':') else {
            return Err(format!("expected `verb: value`, got {line:?}").into());
        };
        match verb.trim() {
            // one space after the colon is the separator; the rest is typed as is
            "wait" => terminal.wait_for(value.trim(), case)?,
            "key" => terminal.send(&key(value.trim())?)?,
            "text" => terminal.send(value.strip_prefix(' ').unwrap_or(value).as_bytes())?,
            // a paste arrives between the bracketed-paste markers, which is what
            // tells the app it is one string and not a hundred key presses
            "paste" => terminal.send(
                format!(
                    "\u{1b}[200~{}\u{1b}[201~",
                    value.strip_prefix(' ').unwrap_or(value)
                )
                .as_bytes(),
            )?,
            other => return Err(format!("unknown step: {other}").into()),
        }
    }
    terminal.settle();

    // the write lands when the app gets to it, so the file is polled
    let expected = case.expect.clone();
    let input = case.target().to_path_buf();
    let screen = terminal.screen();
    eventually(
        || match &expected {
            Some(text) => expect(
                &fs::read_to_string(&input).ok(),
                &Some(text.clone()),
                &format!("TODOs.md\n  screen:\n{screen}"),
            ),
            None => expect(
                &fs::read_to_string(&input).ok().map(String::into_bytes),
                &case.before.clone(),
                &format!("TODOs.md unchanged\n  screen:\n{screen}"),
            ),
        },
        WAIT,
    )?;

    for wanted in &tui.screens {
        expect_contains(&terminal.screen(), wanted, "the finished screen")?;
    }
    Ok(())
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
        "ctrl-f" => "\u{6}",
        // a command key, as the kitty keyboard protocol encodes it: CSI 102 ; 9 u
        // (modifier 9 is 1 + the SUPER bit)
        "cmd-f" => "\u{1b}[102;9u",
        "ctrl-u" => "\u{15}",
        // what a terminal sends for opt+arrow: the modifier is in the sequence
        "alt-up" => "\u{1b}[1;3A",
        "alt-down" => "\u{1b}[1;3B",
        // the same key with other modifiers: which one a terminal sends for
        // opt+arrow depends on the terminal
        "meta-down" => "\u{1b}[1;9B",
        "shift-down" => "\u{1b}[1;2B",
        "ctrl-down" => "\u{1b}[1;5B",
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
        let mut command = CommandBuilder::new(env!("CARGO_BIN_EXE_lazytodo"));
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
        let mut last = self.screen();
        for _ in 0..50 {
            std::thread::sleep(Duration::from_millis(20));
            let now = self.screen();
            if now == last {
                return;
            }
            last = now;
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
