//! The only place an `Action` turns into disk or terminal state.

use crate::state::{Action, App};
use std::io::Write;

use aricma_todo_lib::error::Error;
use aricma_todo_lib::store;

use crate::helpers::base64;

pub fn perform(app: &mut App, action: Action) {
    match action {
        Action::Save { text } => match store::save(&app.baseline, &text) {
            Ok(saved) => app.saved(saved),
            // the file moved under the write: the edit is still in memory, and only
            // the person in front of it can say which version to keep
            Err(error @ Error::StaleFile(_)) => {
                let _ = error;
                app.conflict();
            }
            Err(error) => app.set_error(error.to_string()),
        },
        Action::Overwrite { text } => match store::overwrite(&app.baseline, &text) {
            Ok(saved) => app.saved(saved),
            Err(error) => app.set_error(error.to_string()),
        },
        Action::Copy { text } => copy(&text),
        Action::Quit => app.quit = true,
    }
}

/// Put text on the clipboard: OSC 52, which is the only clipboard a program on the
/// other side of a pty can reach, and on a local macOS session the system's own —
/// Terminal.app ignores the sequence, and there is no way to find that out.
///
/// Over ssh the clipboard that matters is the far one, so only the sequence goes
/// out: `pbcopy` there would fill the wrong machine.
fn copy(text: &str) {
    let sequence = format!("\u{1b}]52;c;{}\u{1b}\\", base64(text.as_bytes()));
    let mut out = std::io::stdout();
    let _ = out.write_all(sequence.as_bytes());
    let _ = out.flush();
    #[cfg(target_os = "macos")]
    if local(
        std::env::var_os("SSH_CONNECTION").is_some(),
        std::env::var_os("SSH_TTY").is_some(),
    ) {
        let _ = pbcopy(text);
    }
}

/// Whether the screen in front of this process is on this machine.
#[cfg(target_os = "macos")]
fn local(ssh: bool, ssh_tty: bool) -> bool {
    !ssh && !ssh_tty
}

#[cfg(target_os = "macos")]
fn pbcopy(text: &str) -> std::io::Result<()> {
    use std::process::{Command, Stdio};
    let mut child = Command::new("pbcopy").stdin(Stdio::piped()).spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(text.as_bytes())?;
    }
    child.wait()?;
    Ok(())
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use super::{local, pbcopy};

    #[test]
    fn a_local_session_uses_it_and_an_ssh_one_does_not() {
        assert!(local(false, false));
        assert!(!local(true, false));
        assert!(!local(false, true));
    }

    #[test]
    fn pbcopy_reads_the_text_off_stdin() {
        let dir = tempfile::tempdir().unwrap();
        let fake = dir.path().join("pbcopy");
        let caught = dir.path().join("clip");
        std::fs::write(
            &fake,
            format!("#!/bin/sh\n/bin/cat > {}\n", caught.display()),
        )
        .unwrap();
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();

        // the only test that touches PATH: all it can change is what a subprocess
        // lookup finds, and it is put back before anything is asserted
        let before = std::env::var_os("PATH");
        std::env::set_var("PATH", dir.path());
        let copied = pbcopy("- [ ] pay rent #home");
        std::env::set_var("PATH", before.unwrap_or_default());

        copied.unwrap();
        assert_eq!(
            std::fs::read_to_string(&caught).unwrap(),
            "- [ ] pay rent #home"
        );
    }
}
