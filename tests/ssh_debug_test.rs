//! Manual SSH connectivity/debugging tests against a *real* remote host.
//!
//! These are not run by `cargo test` by default (there's no fake SSH server
//! here) — they are `#[ignore]`d and meant to be run by hand when diagnosing
//! networking problems (auth, latency, firewalls, remote `zfs` availability,
//! etc). They deliberately reuse the exact `SshSession` code paths from
//! `src/ssh.rs` that `main.rs::process_pool` uses in production, so a
//! failure here points at the same place a real backup run would fail.
//!
//! Run the connectivity check:
//!
//! ```text
//! ST_TEST_SSH_HOST=root@backup.example.com \
//!     cargo test --test ssh_debug_test ssh_connect_and_list -- --ignored --nocapture
//! ```
//!
//! Optionally restrict the `zfs list` to one pool:
//!
//! ```text
//! ST_TEST_SSH_HOST=root@backup.example.com ST_TEST_SSH_POOL=tank \
//!     cargo test --test ssh_debug_test ssh_connect_and_list -- --ignored --nocapture
//! ```
//!
//! To also smoke-test the `zfs send` streaming path (the code that pipes
//! remote `zfs send` stdout into a local `zfs recv`), point it at an
//! existing snapshot:
//!
//! ```text
//! ST_TEST_SSH_HOST=root@backup.example.com ST_TEST_SSH_SNAPSHOT=tank/data@test \
//!     cargo test --test ssh_debug_test ssh_stream_zfs_send -- --ignored --nocapture
//! ```

use std::time::Instant;
use st_zfs_send_recv::ssh::SshSession;

fn test_host() -> String {
    std::env::var("ST_TEST_SSH_HOST").expect(
        "set ST_TEST_SSH_HOST=user@host to run this test, e.g.:\n\
         ST_TEST_SSH_HOST=root@backup.example.com cargo test --test ssh_debug_test -- --ignored --nocapture",
    )
}

/// Exercises the same sequence as `main.rs::process_pool`: connect, check
/// remote root/sudo privileges, then run the `zfs list` command used by
/// `backup::discover_remote_fs`. Prints timing for each stage so slow steps
/// (DNS, SSH handshake/auth, remote `zfs` startup) are easy to tell apart.
#[tokio::test]
#[ignore = "requires a real SSH host; set ST_TEST_SSH_HOST and run with --ignored"]
async fn ssh_connect_and_list() {
    let host = test_host();
    let pool = std::env::var("ST_TEST_SSH_POOL").unwrap_or_default();

    eprintln!("== ssh_connect_and_list == connecting to {host}");

    let t0 = Instant::now();
    let mut ssh = SshSession::new(&host).await.expect("SSH connection failed");
    eprintln!("connected in {:?}", t0.elapsed());

    let t1 = Instant::now();
    let has_privileges = ssh
        .check_root_privileges()
        .await
        .expect("privilege check failed");
    eprintln!(
        "privilege check in {:?}: root_or_sudo={} uses_sudo={}",
        t1.elapsed(),
        has_privileges,
        ssh.uses_sudo()
    );
    assert!(
        has_privileges,
        "remote user on {} is not root and sudo is not available",
        ssh.host()
    );

    let t2 = Instant::now();
    let mut args = vec![
        "list", "-Hpr", "-t", "filesystem,volume", "-o",
        "guid,createtxg,creation,type,used,name", "-s", "name",
    ];
    if !pool.is_empty() {
        args.push(&pool);
    }
    let output = ssh
        .execute_zfs_list(&args)
        .await
        .expect("zfs list over SSH failed");
    eprintln!(
        "zfs list in {:?}, {} line(s) returned",
        t2.elapsed(),
        output.lines().count()
    );
    for line in output.lines().take(5) {
        eprintln!("  {line}");
    }

    eprintln!("== ssh_connect_and_list == total {:?}", t0.elapsed());
}

/// Exercises `SshSession::spawn_zfs_send`, the streaming code path used by
/// `backup::perform_incremental_backup` to pipe a remote `zfs send` directly
/// into a local `zfs recv`. Reads just the first chunk of the stream to
/// confirm data is flowing (and how long it takes to arrive) without
/// transferring or receiving a whole snapshot.
#[tokio::test]
#[ignore = "requires a real SSH host and an existing snapshot; set ST_TEST_SSH_HOST and ST_TEST_SSH_SNAPSHOT"]
async fn ssh_stream_zfs_send() {
    use tokio::io::AsyncReadExt;

    let host = test_host();
    let snapshot = std::env::var("ST_TEST_SSH_SNAPSHOT").expect(
        "set ST_TEST_SSH_SNAPSHOT=pool/fs@snap to run this test",
    );

    eprintln!("== ssh_stream_zfs_send == connecting to {host}");
    let ssh = SshSession::new(&host).await.expect("SSH connection failed");

    let t0 = Instant::now();
    let mut child = ssh
        .spawn_zfs_send(&["send", &snapshot])
        .await
        .expect("failed to spawn zfs send");
    eprintln!("zfs send spawned in {:?}", t0.elapsed());

    let mut stdout = child
        .stdout()
        .take()
        .expect("zfs send child is missing stdout");

    let t1 = Instant::now();
    let mut buf = vec![0u8; 64 * 1024];
    let n = stdout
        .read(&mut buf)
        .await
        .expect("failed reading from zfs send stream");
    eprintln!(
        "received first {} byte(s) of the send stream in {:?}",
        n,
        t1.elapsed()
    );
    assert!(n > 0, "zfs send produced no data for {snapshot}");

    // Deliberately don't drain the rest of the stream or wait for the
    // child - this is a connectivity smoke test, not a full backup. Dropping
    // `child` tears down the local ssh process (it does not kill the remote
    // `zfs send`, which will just see its pipe close and exit).
}
