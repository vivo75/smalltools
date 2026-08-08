// Confronta questo file con `../../../src/error.rs` nel progetto reale:
// è quasi identico, ridotto solo a poche varianti per restare leggibile.

use std::io;
use thiserror::Error;

/// Un `enum` di errori "a la libreria": ogni variante rappresenta un
/// modo *specifico e nominato* in cui questa parte del programma può
/// fallire. È l'equivalente Rust di definire una gerarchia di eccezioni
/// custom in Python:
///
/// ```python
/// class ZfsBackupError(Exception): pass
/// class ConfigError(ZfsBackupError): pass
/// class IoError(ZfsBackupError): pass
/// ```
///
/// con una differenza enorme: qui non è una gerarchia con `isinstance`,
/// è un singolo tipo con varianti chiuse (closed set). Il compilatore
/// sa esattamente quali sono TUTTI i modi in cui una funzione che
/// restituisce `ZfsBackupError` può fallire, e un `match` su di esso
/// deve gestirli tutti (o dichiarare esplicitamente `_ => ...`).
#[derive(Error, Debug)]
#[allow(dead_code)] // `InvalidSnapshot` non è ancora usata in questo step ridotto
pub enum ZfsBackupError {
    /// `#[error("...")]` genera l'implementazione di `Display` (quello
    /// che vedi quando fai `println!("{err}")`). Non serve scrivere a
    /// mano un `__str__` come in Python.
    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Invalid snapshot state: {0}")]
    InvalidSnapshot(String),

    /// `#[from]` genera automaticamente `impl From<io::Error> for
    /// ZfsBackupError`. Questo è ciò che rende possibile scrivere `?`
    /// su un'operazione che restituisce `io::Error` dentro a una
    /// funzione che restituisce `ZfsResult<T>`: la conversione avviene
    /// da sola. È l'equivalente di Python:
    ///
    /// ```python
    /// try:
    ///     ...
    /// except OSError as e:
    ///     raise ZfsBackupError(str(e)) from e
    /// ```
    ///
    /// ma senza scrivere il `try/except` a mano ogni volta.
    #[error("IO error: {0}")]
    IoError(#[from] io::Error),
}

/// Un type alias per non ripetere `Result<T, ZfsBackupError>` ovunque.
/// Equivalente concettuale a definire un tipo `Result` specializzato
/// come si farebbe con un generic type alias in un file `.pyi`, ma qui
/// è usato attivamente nel codice, non solo nei type stub.
pub type ZfsResult<T> = Result<T, ZfsBackupError>;
