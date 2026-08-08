// Quasi identico a `../../../src/config.rs` del progetto reale: qui
// manca solo `tracing::debug!` (arriverà nello step 9) e i test sono
// leggermente ampliati per illustrare più casi.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// `#[derive(Deserialize)]` genera, a compile-time, tutto il codice che
/// in Python scriveresti a mano con `pydantic` o con
/// `yaml.safe_load(...)` + validazione manuale campo per campo.
/// Nessuna riflessione a runtime: `serde` ispeziona la struct una sola
/// volta, durante la compilazione, e genera codice specializzato per
/// *questa* struct — è uno dei motivi per cui il parsing in Rust è
/// tipicamente molto più veloce che in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    // `#[serde(skip)]`: questo campo non viene letto dal YAML. Lo
    // popoliamo noi dopo, dal nome del file. Equivalente Python:
    // un campo che il tuo `pydantic.BaseModel` esclude esplicitamente
    // da `Config.fields` e assegni manualmente dopo `.parse_obj(...)`.
    #[serde(skip)]
    pub name: String,

    pub remote_host: String,
    pub remote_pool: String,
    pub local_pool: String,

    // `#[serde(default = "...")]`: se la chiave manca nello YAML,
    // chiama questa funzione per ottenere il valore. Equivalente a
    // `field: str = "lz4"` in una dataclass, o a `data.get("compress",
    // "lz4")` se stessi facendo il parsing a mano.
    #[serde(default = "default_compress")]
    pub compress: String,

    #[serde(default = "default_compress_flags")]
    pub compress_flags: String,

    #[serde(default = "default_uncompress_flags")]
    pub uncompress_flags: String,

    // `#[serde(default)]` senza argomenti usa `Default::default()` del
    // tipo del campo: per un `Vec<T>` è il vettore vuoto. Corrisponde
    // a `field(default_factory=list)` di una dataclass Python — e non
    // a caso: lo stesso identico problema (un `Vec::new()`/`list()`
    // condiviso per sbaglio tra istanze) è ciò che entrambe le sintassi
    // "verbose" evitano.
    #[serde(default)]
    pub keep_snapshots: Vec<KeepPolicy>,

    #[serde(default)]
    pub skip_filesystems: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeepPolicy {
    pub count: u32,
    pub label: String,
}

fn default_compress() -> String {
    "lz4".to_string()
}

fn default_compress_flags() -> String {
    String::new()
}

fn default_uncompress_flags() -> String {
    "-dcfm".to_string()
}

/// Scansiona una directory cercando file `*.pool` e li carica tutti.
/// Confronta con l'equivalente Python:
///
/// ```python
/// def load_pool_configs(config_dir: Path) -> list[PoolConfig]:
///     if not config_dir.exists():
///         return []
///     configs = []
///     for path in sorted(config_dir.iterdir()):
///         if not path.is_file() or path.suffix != ".pool":
///             continue
///         config = load_pool_config(path)
///         config.name = path.stem
///         configs.append(config)
///     return configs
/// ```
///
/// La struttura è quasi identica riga per riga: questo non è un caso.
/// La logica "attraversa una directory, filtra, trasforma, accumula" è
/// la stessa in qualunque linguaggio; quello che cambia è *dove* il
/// linguaggio ti protegge da errori (qui: se dimentichi di gestire un
/// `Result`, non compila; in Python, un'eccezione non gestita in
/// `load_pool_config` fa fallire l'intera scansione della directory
/// invece di essere segnalata per quel singolo file).
pub fn load_pool_configs(config_dir: &Path) -> Result<Vec<PoolConfig>> {
    let mut configs = Vec::new();

    if !config_dir.exists() {
        return Ok(configs);
    }

    for entry in std::fs::read_dir(config_dir).context("Failed to read config directory")? {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        // `Option<&OsStr>` invece di un `str | None` implicito: capire
        // se un path ha un'estensione richiede di gestire
        // esplicitamente "non ce l'ha proprio" (`None`) separatamente
        // da "ce l'ha ma non è quella giusta" (`Some(altro)`).
        if let Some(extension) = path.extension() {
            if extension != "pool" {
                continue;
            }
        } else {
            continue;
        }

        let mut config = load_pool_config(&path)?;

        if let Some(filename) = path.file_stem() {
            config.name = filename.to_string_lossy().to_string();
        }

        configs.push(config);
    }

    Ok(configs)
}

fn load_pool_config(path: &Path) -> Result<PoolConfig> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {:?}", path))?;

    // Una sola riga per fare quello che in Python richiederebbe
    // `yaml.safe_load()` + validare a mano che ogni chiave attesa
    // esista e abbia il tipo giusto. Se lo YAML ha un campo mancante
    // senza default, o un tipo sbagliato (es. `count: "sette"` invece
    // di un intero), `serde_yaml::from_str` restituisce `Err` con un
    // messaggio che indica riga e colonna del problema.
    let config: PoolConfig = serde_yaml::from_str(&content)
        .with_context(|| format!("Failed to parse config file: {:?}", path))?;

    validate_pool_config(&config)?;

    Ok(config)
}

fn validate_pool_config(config: &PoolConfig) -> Result<()> {
    if config.remote_host.is_empty() {
        anyhow::bail!("remote_host is required");
    }
    if config.remote_pool.is_empty() {
        anyhow::bail!("remote_pool is required");
    }
    if config.local_pool.is_empty() {
        anyhow::bail!("local_pool is required");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_parsing() {
        let yaml = r#"
remote_host: backup.example.com
remote_pool: tank
local_pool: backup/tank
compress: lz4
keep_snapshots:
  - count: 7
    label: daily
  - count: 4
    label: weekly
        "#;

        let config: PoolConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.remote_host, "backup.example.com");
        assert_eq!(config.remote_pool, "tank");
        assert_eq!(config.local_pool, "backup/tank");
        assert_eq!(config.keep_snapshots.len(), 2);
    }

    #[test]
    fn defaults_are_applied_when_fields_are_missing() {
        let yaml = r#"
remote_host: backup.example.com
remote_pool: tank
local_pool: backup/tank
        "#;

        let config: PoolConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.compress, "lz4");
        assert_eq!(config.uncompress_flags, "-dcfm");
        assert!(config.keep_snapshots.is_empty());
        assert!(config.skip_filesystems.is_empty());
    }

    #[test]
    fn missing_required_field_fails_to_parse() {
        // manca `local_pool`: serde_yaml segnala l'errore prima ancora
        // che arriviamo alla nostra validazione manuale.
        let yaml = r#"
remote_host: backup.example.com
remote_pool: tank
        "#;

        let result: Result<PoolConfig, _> = serde_yaml::from_str(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn empty_local_pool_fails_manual_validation() {
        let config = PoolConfig {
            name: "test".to_string(),
            remote_host: "host".to_string(),
            remote_pool: "tank".to_string(),
            local_pool: String::new(),
            compress: default_compress(),
            compress_flags: default_compress_flags(),
            uncompress_flags: default_uncompress_flags(),
            keep_snapshots: vec![],
            skip_filesystems: vec![],
        };

        assert!(validate_pool_config(&config).is_err());
    }

    #[test]
    fn load_pool_configs_scans_directory_and_sets_name_from_filename() {
        let dir = tempfile::tempdir().unwrap();

        std::fs::write(
            dir.path().join("tank.pool"),
            "remote_host: h1\nremote_pool: tank\nlocal_pool: backup/tank\n",
        )
        .unwrap();

        // Un file senza estensione `.pool` viene ignorato.
        std::fs::write(dir.path().join("README.txt"), "not a config").unwrap();

        let configs = load_pool_configs(dir.path()).unwrap();

        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].name, "tank");
        assert_eq!(configs[0].remote_pool, "tank");

        // `dir` (una `TempDir`) cancella la cartella temporanea quando
        // esce di scope qui, a fine test: il suo `Drop` fa pulizia da
        // solo, come un `with tempfile.TemporaryDirectory():` in
        // Python, ma senza bisogno del blocco `with` — basta che il
        // valore esca di scope.
    }

    #[test]
    fn load_pool_configs_on_missing_directory_returns_empty_vec() {
        let configs = load_pool_configs(Path::new("/does/not/exist/at/all")).unwrap();
        assert!(configs.is_empty());
    }
}
