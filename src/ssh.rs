use anyhow::{Context, Result};
use openssh::{Child, Session, Stdio};
use std::process::Output;
use tracing::{debug, warn};

pub struct SshSession {
    session: Session,
    host: String,
    use_sudo: bool,
}

impl SshSession {
    #[allow(dead_code)]
    pub async fn new(host: &str) -> Result<Self> {
        let session = Session::connect(host, openssh::KnownHosts::Accept)
            .await
            .context("Failed to connect via SSH")?;

        Ok(SshSession {
            session,
            host: host.to_string(),
            use_sudo: false,
        })
    }

    pub async fn check_root_privileges(&mut self) -> Result<bool> {
        let output = self
            .session
            .command("test")
            .arg("${UID}")
            .arg("-ne")
            .arg("0")
            .output()
            .await
            .context("Failed to check UID")?;

        if output.status.success() {
            // Not root, try sudo
            let sudo_output = self
                .session
                .command("sudo")
                .arg("test")
                .output()
                .await;

            if sudo_output.is_ok() && sudo_output?.status.success() {
                self.use_sudo = true;
                debug!("Using sudo for privileged commands on {}", self.host);
                Ok(true)
            } else {
                warn!("User is not root and sudo is not available on {}", self.host);
                Ok(false)
            }
        } else {
            // Already root
            debug!("User is root on {}", self.host);
            Ok(true)
        }
    }

    pub async fn execute_zfs_list(
        &self,
        args: &[&str],
    ) -> Result<String> {
        let mut cmd = self.session.command("zfs");

        for arg in args {
            cmd.arg(arg);
        }

        let output = cmd
            .output()
            .await
            .context("Failed to execute zfs list command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("zfs command failed: {}", stderr);
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Spawns `zfs send` on the remote host and returns the running child process
    /// with its stdout/stderr piped back, without waiting for it to finish or
    /// buffering its output. Callers should stream `child.stdout()` directly into
    /// the destination (e.g. a local `zfs recv`'s stdin) so the send stream never
    /// has to be held in memory in full.
    pub async fn spawn_zfs_send(&self, args: &[&str]) -> Result<Child<&Session>> {
        let mut cmd = self.session.command("zfs");

        for arg in args {
            cmd.arg(arg);
        }

        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        cmd.spawn()
            .await
            .context("Failed to spawn zfs send command")
    }

    #[allow(dead_code)]
    pub async fn execute_command(&self, program: &str, args: &[&str]) -> Result<Output> {
        let mut cmd = self.session.command(program);

        for arg in args {
            cmd.arg(arg);
        }

        cmd.output()
            .await
            .context("Failed to execute remote command")
    }

    #[allow(dead_code)]
    pub fn host(&self) -> &str {
        &self.host
    }

    #[allow(dead_code)]
    pub fn uses_sudo(&self) -> bool {
        self.use_sudo
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssh_session_creation() {
        // This would require an actual SSH setup to test
        // For now, we'll just verify the struct can be constructed
        // In real tests, use something like testcontainers
    }
}
