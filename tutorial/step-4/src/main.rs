mod config;

fn main() -> anyhow::Result<()> {
    // Simuliamo /etc/st-zfs-send-recv.d/ con una directory temporanea,
    // così questo esempio gira senza bisogno di permessi di root o di
    // creare file di sistema veri.
    let dir = tempfile::tempdir()?;

    std::fs::write(
        dir.path().join("tank.pool"),
        r#"
remote_host: backup.example.com
remote_pool: tank
local_pool: backup/tank
keep_snapshots:
  - count: 7
    label: daily
"#,
    )?;

    std::fs::write(
        dir.path().join("media.pool"),
        r#"
remote_host: backup.example.com
remote_pool: media
local_pool: backup/media
compress: zstd
skip_filesystems:
  - "^media/scratch$"
"#,
    )?;

    let configs = config::load_pool_configs(dir.path())?;

    println!("Loaded {} pool configuration(s):", configs.len());
    for cfg in &configs {
        println!(
            "  - {}: {} ({}) -> {} [compress={}]",
            cfg.name, cfg.remote_host, cfg.remote_pool, cfg.local_pool, cfg.compress
        );
    }

    Ok(())
}
