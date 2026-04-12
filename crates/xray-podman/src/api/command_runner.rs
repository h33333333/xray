use std::process::{Child, Command, Output, Stdio};

use anyhow::Context;

use crate::Result;

/// A convenient wrapper around [Command] to invoke Podman-related commands.
pub(crate) struct CommandRunner {
    child: Child,
}

impl CommandRunner {
    /// Spawns a new CLI command using options from the provided [CommandBuilder].
    pub(crate) fn spawn(command: &str, args: &[&str]) -> Result<Self> {
        let mut command = Command::new(command);
        for arg in args {
            command.arg(arg);
        }
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        let child = command
            .spawn()
            .context("failed to run a podman CLI command")?;

        Ok(CommandRunner { child })
    }

    /// Waits until the wrapper shell command exits and returns its [Output].
    pub(crate) fn wait_until_completion(self) -> Result<Output> {
        self.child
            .wait_with_output()
            .context("child process failed")
            .map_err(Into::into)
    }
}
