use std::process::{Command, Stdio};
use tracing::warn;

// Launch and forget.
pub fn detached(program: &str, args: &[&str]) {
    match Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(mut child) => {
            std::thread::spawn(move || {
                let _ = child.wait();
            });
        }
        Err(e) => warn!(%e, program, "could not launch"),
    }
}
