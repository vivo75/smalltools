// La parte "nuova" di questo step, non ancora vista: filtrare i
// filesystem con `skip_filesystems` (regex, come in
// `../../../src/config.rs`), e la logica di decisione a tre vie di
// `../../../src/backup.rs::backup_filesystem`.

use crate::error::ZfsBackupError;
use regex::Regex;
use tracing::{error, info};

/// Compila i pattern `skip_filesystems` in `Regex` una sola volta, poi
/// filtra la lista dei filesystem scoperti. Confronta con l'equivalente
/// Python:
///
/// ```python
/// import re
/// patterns = [re.compile(p) for p in skip_filesystems]
/// kept = [name for name in names if not any(p.search(name) for p in patterns)]
/// ```
///
/// La forma è la stessa: compila una volta, poi `filter`/list
/// comprehension con `any(...)`. La differenza è che qui il
/// compilatore garantisce che `kept` sia sempre un `Vec<String>` — non
/// un `Vec` di tipi misti — e che un pattern regex malformato produca
/// un `Err` tipizzato (`ZfsBackupError::InvalidPattern`) invece di
/// un'eccezione `re.error` generica sollevata a un punto qualunque del
/// programma.
pub fn filter_filesystems(
    names: Vec<String>,
    skip_patterns: &[String],
) -> Result<Vec<String>, ZfsBackupError> {
    let patterns: Vec<Regex> = skip_patterns
        .iter()
        .map(|pattern| {
            Regex::new(pattern).map_err(|source| ZfsBackupError::InvalidPattern {
                pattern: pattern.clone(),
                source,
            })
        })
        .collect::<Result<_, _>>()?;

    // `.into_iter()` invece di `.iter()`: consuma `names` per produrre
    // `String` possedute nel risultato, invece di prestiti che
    // vivrebbero solo quanto `names`. `.retain()` sarebbe l'alternativa
    // che modifica il Vec sul posto; `.filter().collect()` è più
    // idiomatico quando, come qui, si vuole restituire un nuovo Vec e
    // si preferisce comporre l'operazione come una catena di iteratori.
    let kept = names
        .into_iter()
        .filter(|name| !patterns.iter().any(|re| re.is_match(name)))
        .collect();

    Ok(kept)
}

/// Rispecchia i tre rami di `backup_filesystem` nel codice reale.
/// Un `enum` con dati associati esprime "questi sono gli UNICI tre
/// esiti possibili" in modo che il `match` che lo consuma sia
/// esaustivo — impossibile dimenticare un caso, cosa che un
/// `if`/`elif`/`else` a catena in Python non garantisce.
pub enum BackupDecision {
    UpToDate,
    NeedsInitialization,
    Incremental { from: String, to: String },
    NothingToSend,
}

pub fn decide(common: Option<String>, newest: Option<String>) -> BackupDecision {
    match (common, newest) {
        (Some(c), Some(n)) if c == n => BackupDecision::UpToDate,
        (None, Some(_)) => BackupDecision::NeedsInitialization,
        (Some(from), Some(to)) => BackupDecision::Incremental { from, to },
        (_, None) => BackupDecision::NothingToSend,
    }
}

pub fn log_decision(fs_name: &str, decision: &BackupDecision) {
    match decision {
        BackupDecision::UpToDate => info!("{fs_name} is already up to date"),
        BackupDecision::NeedsInitialization => {
            error!("{fs_name} has no common snapshot: needs manual initialization")
        }
        BackupDecision::Incremental { from, to } => info!("{fs_name}{from} => {to}"),
        BackupDecision::NothingToSend => info!("{fs_name} has no snapshots on the remote side"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_out_matching_filesystems() {
        let names = vec![
            "tank/data".to_string(),
            "tank/docker".to_string(),
            "tank/temp-1".to_string(),
        ];
        let skip = vec!["^tank/docker$".to_string(), "^tank/temp".to_string()];

        let kept = filter_filesystems(names, &skip).unwrap();
        assert_eq!(kept, vec!["tank/data".to_string()]);
    }

    #[test]
    fn invalid_pattern_is_a_typed_error() {
        let err =
            filter_filesystems(vec!["tank/data".to_string()], &["(".to_string()]).unwrap_err();
        assert!(matches!(err, ZfsBackupError::InvalidPattern { .. }));
    }

    #[test]
    fn decision_up_to_date() {
        let d = decide(Some("@s1".to_string()), Some("@s1".to_string()));
        assert!(matches!(d, BackupDecision::UpToDate));
    }

    #[test]
    fn decision_needs_initialization() {
        let d = decide(None, Some("@s1".to_string()));
        assert!(matches!(d, BackupDecision::NeedsInitialization));
    }

    #[test]
    fn decision_incremental() {
        let d = decide(Some("@s1".to_string()), Some("@s2".to_string()));
        match d {
            BackupDecision::Incremental { from, to } => {
                assert_eq!(from, "@s1");
                assert_eq!(to, "@s2");
            }
            _ => panic!("expected Incremental"),
        }
    }

    #[test]
    fn decision_nothing_to_send() {
        let d = decide(None, None);
        assert!(matches!(d, BackupDecision::NothingToSend));
    }
}
