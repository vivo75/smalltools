use crate::error::ZfsBackupError;
use crate::util;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeepPolicy {
    pub count: u32,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolConfig {
    pub name: String,
    pub remote_host: String,
    pub remote_pool: String,
    pub local_pool: String,
    pub keep_snapshots: Vec<KeepPolicy>,
}

/// `crate::util::normalize_pool_name` è raggiungibile da qui perché
/// `config` e `util` sono entrambi moduli della STESSA crate — è
/// esattamente il caso d'uso per cui esiste `pub(crate)`: condividere
/// codice interno tra moduli-fratelli senza esporlo all'esterno.
pub fn new_pool_config(
    name: &str,
    remote_host: &str,
    remote_pool: &str,
    local_pool: &str,
) -> PoolConfig {
    PoolConfig {
        name: util::normalize_pool_name(name),
        remote_host: remote_host.to_string(),
        remote_pool: remote_pool.to_string(),
        local_pool: local_pool.to_string(),
        keep_snapshots: Vec::new(),
    }
}

pub fn validate_pool_config(config: &PoolConfig) -> Result<(), ZfsBackupError> {
    if config.remote_host.is_empty() {
        return Err(ZfsBackupError::ConfigError(
            "remote_host is required".to_string(),
        ));
    }
    if config.remote_pool.is_empty() {
        return Err(ZfsBackupError::ConfigError(
            "remote_pool is required".to_string(),
        ));
    }
    if config.local_pool.is_empty() {
        return Err(ZfsBackupError::ConfigError(
            "local_pool is required".to_string(),
        ));
    }
    Ok(())
}

// Questo modulo di test è compilato SOLO quando lanci `cargo test`, ed
// è compilato *come parte della stessa crate* di `config.rs`: perciò
// può chiamare `util::normalize_pool_name` (pub(crate)) direttamente,
// e persino ispezionare dettagli privati se ce ne fossero. Questa è la
// differenza fondamentale con `tests/integration_test.rs`: quello è
// una crate SEPARATA che collega la nostra libreria come dipendenza
// esterna, e vede solo ciò che è `pub`.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_pool_config_normalizes_name() {
        let config = new_pool_config("  TANK ", "host", "tank", "backup/tank");
        assert_eq!(config.name, "tank");
    }

    #[test]
    fn can_call_pub_crate_helper_directly_from_unit_test() {
        // Accesso diretto a una funzione pub(crate): impossibile da
        // tests/integration_test.rs, possibile qui perché siamo nella
        // stessa crate.
        assert_eq!(util::normalize_pool_name("Tank"), "tank");
    }

    #[test]
    fn empty_remote_pool_is_rejected() {
        let mut config = new_pool_config("tank", "host", "tank", "backup/tank");
        config.remote_pool = String::new();
        let err = validate_pool_config(&config).unwrap_err();
        assert_eq!(
            err,
            ZfsBackupError::ConfigError("remote_pool is required".to_string())
        );
    }
}
