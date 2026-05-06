/// Utilities for running shell commands and capturing output.
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

use anyhow::Result;

use crate::manifest::CommandResult;

const OUTPUT_TAIL_LINES: usize = 40;

/// Execute a shell command in `working_dir` with optional extra env vars.
/// Returns a [`CommandResult`] regardless of whether the command succeeds,
/// so callers can always report what happened.
pub fn run_command(
    cmd: &str,
    working_dir: &Path,
    env: &HashMap<String, String>,
) -> Result<CommandResult> {
    let mut command = if cfg!(target_os = "windows") {
        let mut c = Command::new("cmd");
        c.args(["/C", cmd]);
        c
    } else {
        let mut c = Command::new("sh");
        c.args(["-c", cmd]);
        c
    };

    command.current_dir(working_dir);
    for (k, v) in env {
        command.env(k, v);
    }

    let output = command
        .output()
        .map_err(|e| anyhow::anyhow!("failed to spawn command `{}`: {}", cmd, e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let combined = format!("{}{}", stdout, stderr);
    let tail = tail_lines(&combined, OUTPUT_TAIL_LINES);

    Ok(CommandResult {
        command: cmd.to_owned(),
        success: output.status.success(),
        exit_code: output.status.code(),
        output_tail: tail,
    })
}

fn tail_lines(s: &str, n: usize) -> String {
    let lines: Vec<&str> = s.lines().collect();
    if lines.len() <= n {
        s.to_owned()
    } else {
        lines[lines.len() - n..].join("\n")
    }
}
