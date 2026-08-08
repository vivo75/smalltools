// step-8: async/await. Il vero `../../src/ssh.rs` apre una sessione
// SSH e ci esegue comandi `zfs` asincroni; qui non abbiamo un host SSH
// vero a disposizione, quindi simuliamo la stessa forma — un comando
// esterno lanciato in modo asincrono, con latenza artificiale per
// imitare un round-trip di rete — usando `echo`/`cat` locali al posto
// di `zfs`/`ssh`. La struttura del codice, e le lezioni su ownership e
// concorrenza, sono identiche a quelle del codice reale.

use anyhow::{Context, Result};
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;

/// Corrisponde a `SshSession::execute_zfs_list` in `../../src/ssh.rs`:
/// lancia un comando esterno e ne raccoglie l'output. La differenza è
/// solo `tokio::process::Command` (locale) invece di
/// `openssh::Session::command` (remoto via SSH) — la forma della
/// funzione, `async fn ... -> Result<String>`, è la stessa.
///
/// `async fn` non esegue nulla quando viene chiamata: restituisce
/// immediatamente un `Future`, un valore che rappresenta "questo
/// lavoro, non ancora iniziato". Solo `.await` lo fa effettivamente
/// avanzare. È l'equivalente esatto di una coroutine Python creata da
/// `async def` e mai eseguita finché non la passi a `await` o
/// `asyncio.gather`.
async fn run_remote(program: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(program)
        .args(args)
        .output() // Future: non ha ancora eseguito nulla
        .await // qui, e solo qui, il processo viene effettivamente lanciato e atteso
        .with_context(|| format!("failed to run {program}"))?;

    if !output.status.success() {
        anyhow::bail!("{program} exited with {}", output.status);
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Simula `zfs list -Hpr ...` su un host remoto: in produzione questo
/// sarebbe `ssh.execute_zfs_list(&[...]).await` (vedi
/// `../../src/backup.rs::discover_remote_fs`). `tokio::time::sleep`
/// aggiunge una latenza artificiale, per rendere visibile più avanti
/// il vantaggio di eseguire più `discover` in parallelo invece che in
/// sequenza — proprio come un vero round-trip SSH su rete avrebbe un
/// costo fisso indipendente dalla CPU disponibile.
async fn discover_filesystems(pool: String) -> Result<Vec<String>> {
    tokio::time::sleep(Duration::from_millis(150)).await; // stand-in per la latenza di rete
    let output = run_remote("echo", &[&format!("{pool}/data\n{pool}/backups")]).await?;
    Ok(output.lines().map(str::to_string).collect())
}

/// Corrisponde alla parte finale di
/// `../../src/backup.rs::perform_incremental_backup`: scrive dati
/// sullo stdin di un processo figlio e ne legge lo stdout, entrambi in
/// modo asincrono. Nel codice reale il processo figlio è `zfs recv`
/// e i dati sono lo stream binario prodotto da `zfs send`; qui usiamo
/// `cat`, che si limita a restituire in output ciò che riceve in
/// input — sufficiente per osservare il pattern write/read asincrono
/// senza bisogno di un pool ZFS vero.
async fn pipe_through_child(payload: &[u8]) -> Result<Vec<u8>> {
    let mut child = Command::new("cat")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .context("failed to spawn child process")?;

    // `.take()` sposta lo stdin fuori da `Option<ChildStdin>`: dopo
    // questa riga `child.stdin` è `None`. È lo stesso pattern
    // Option+ownership visto negli step precedenti, qui applicato a
    // una risorsa di sistema invece che a un dato di dominio.
    let mut stdin = child.stdin.take().context("child has no stdin")?;
    let mut stdout = child.stdout.take().context("child has no stdout")?;

    // Scriviamo e leggiamo concorrentemente: se scrivessimo tutto
    // PRIMA di leggere, con un payload grande il processo figlio
    // potrebbe bloccarsi perché il suo buffer di stdout è pieno e
    // nessuno lo sta ancora svuotando — un classico deadlock da pipe,
    // identico in Python con `subprocess.Popen` se non si usa
    // `communicate()` o thread separati per stdin/stdout.
    let payload_owned = payload.to_vec();
    let writer = tokio::spawn(async move {
        stdin.write_all(&payload_owned).await?;
        drop(stdin); // chiude lo stdin: segnala EOF al processo figlio
        Ok::<(), anyhow::Error>(())
    });

    let mut collected = Vec::new();
    stdout.read_to_end(&mut collected).await?;

    writer.await??; // primo `?` per il JoinError di tokio, secondo per anyhow::Error
    child.wait().await?;

    Ok(collected)
}

#[tokio::main] // espande in codice equivalente a `asyncio.run(main())` di Python
async fn main() -> Result<()> {
    let pools = vec![
        "tank".to_string(),
        "backup".to_string(),
        "archive".to_string(),
    ];

    // --- Sequenziale: un `.await` alla volta, uno dopo l'altro ---
    let start = Instant::now();
    let mut sequential_results = Vec::new();
    for pool in &pools {
        sequential_results.push(discover_filesystems(pool.clone()).await?);
    }
    let sequential_elapsed = start.elapsed();
    println!(
        "sequential: {sequential_elapsed:?} for {} pools",
        pools.len()
    );

    // --- Concorrente: tutte le `discover_filesystems` partono subito ---
    // `tokio::spawn` richiede che la future passata sia `'static`
    // (non prenda in prestito nulla dallo scope corrente): per questo
    // passiamo `pool.clone()` (una `String` posseduta) invece di
    // `&pool`. È lo stesso principio di ownership dello step 2, qui
    // reso necessario dal fatto che il task pianificato da `tokio::spawn`
    // può sopravvivere più a lungo dello scope che l'ha creato.
    let start = Instant::now();
    let handles: Vec<_> = pools
        .iter()
        .map(|pool| tokio::spawn(discover_filesystems(pool.clone())))
        .collect();

    let mut concurrent_results = Vec::new();
    for handle in handles {
        concurrent_results.push(handle.await??);
    }
    let concurrent_elapsed = start.elapsed();
    println!(
        "concurrent: {concurrent_elapsed:?} for {} pools",
        pools.len()
    );

    assert!(
        concurrent_elapsed < sequential_elapsed,
        "running discover_filesystems concurrently should be faster than sequentially"
    );

    for (pool, filesystems) in pools.iter().zip(concurrent_results.iter()) {
        println!("{pool}: {filesystems:?}");
    }

    // --- Pipe verso un processo figlio, come zfs send | zfs recv ---
    let echoed = pipe_through_child(b"zfs stream bytes would go here\n").await?;
    println!(
        "child echoed back: {}",
        String::from_utf8_lossy(&echoed).trim()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test] // richiede un runtime async attivo: espande #[test] + avvio del runtime
    async fn run_remote_captures_stdout() {
        let output = run_remote("echo", &["hello"]).await.unwrap();
        assert_eq!(output, "hello");
    }

    #[tokio::test]
    async fn run_remote_fails_on_nonexistent_program() {
        let result = run_remote("this-program-does-not-exist-xyz", &[]).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn discover_filesystems_parses_two_lines() {
        let fs = discover_filesystems("tank".to_string()).await.unwrap();
        assert_eq!(
            fs,
            vec!["tank/data".to_string(), "tank/backups".to_string()]
        );
    }

    #[tokio::test]
    async fn pipe_through_child_round_trips_data() {
        let echoed = pipe_through_child(b"round trip").await.unwrap();
        assert_eq!(echoed, b"round trip");
    }

    #[tokio::test]
    async fn concurrent_discovery_is_faster_than_sequential() {
        let pools = vec!["a".to_string(), "b".to_string(), "c".to_string()];

        let start = Instant::now();
        for pool in &pools {
            discover_filesystems(pool.clone()).await.unwrap();
        }
        let sequential = start.elapsed();

        let start = Instant::now();
        let handles: Vec<_> = pools
            .iter()
            .map(|pool| tokio::spawn(discover_filesystems(pool.clone())))
            .collect();
        for handle in handles {
            handle.await.unwrap().unwrap();
        }
        let concurrent = start.elapsed();

        assert!(concurrent < sequential);
    }
}
