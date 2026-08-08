// step-3: niente eccezioni. Rust modella "questa funzione può fallire"
// nel tipo di ritorno stesso, con `Result<T, E>`. Vediamo come questo
// cambia il modo di scrivere codice rispetto a `try/except`.

mod error;

use error::{ZfsBackupError, ZfsResult};
use std::path::Path;

/// Restituisce `ZfsResult<()>` — cioè `Result<(), ZfsBackupError>`.
/// `()` come "valore di successo" significa "non c'è nulla di
/// interessante da restituire se va bene, ma la funzione PUÒ fallire".
/// Equivalente Python: una funzione che non ha `return` esplicito ma
/// può sollevare un'eccezione.
fn validate_pool_name(name: &str) -> ZfsResult<()> {
    if name.is_empty() {
        // Costruiamo esplicitamente la variante di errore giusta.
        // Non c'è "throw generico di stringa": il chiamante riceve un
        // valore tipizzato che può ispezionare con `match`.
        return Err(ZfsBackupError::ConfigError(
            "pool name must not be empty".to_string(),
        ));
    }
    if name.contains(' ') {
        return Err(ZfsBackupError::ConfigError(format!(
            "pool name '{name}' must not contain spaces"
        )));
    }
    Ok(())
}

/// Legge un file di configurazione e lo restituisce come stringa.
/// `std::fs::read_to_string` restituisce `io::Result<String>`, cioè
/// `Result<String, io::Error>`. L'operatore `?` fa tre cose in una:
///
/// 1. Se il `Result` è `Ok(v)`, si "srotola" e la funzione continua
///    con `v`.
/// 2. Se è `Err(e)`, converte `e` nel tipo di errore della funzione
///    corrente (qui: `io::Error -> ZfsBackupError` grazie a `#[from]`
///    visto in `error.rs`) e fa immediatamente `return Err(...)`.
/// 3. Tutto questo senza il rumore sintattico di un `try/except`.
///
/// Equivalente Python:
///
/// ```python
/// def read_pool_file(path: Path) -> str:
///     try:
///         with open(path) as f:
///             return f.read()
///     except OSError as e:
///         raise ZfsBackupError(str(e)) from e
/// ```
///
/// La differenza principale non è la lunghezza del codice (`?` è solo
/// zucchero sintattico), ma che la *possibilità di fallire* è scritta
/// nel tipo di ritorno `ZfsResult<String>`. Chi chiama questa funzione
/// lo sa leggendo la firma, senza dover ispezionare il corpo o la
/// documentazione per scoprire quali eccezioni può sollevare (in
/// Python le eccezioni non dichiarate sono "invisibili" finché non
/// esplodono a runtime).
fn read_pool_file(path: &Path) -> ZfsResult<String> {
    let content = std::fs::read_to_string(path)?; // <- qui avviene la conversione automatica
    Ok(content)
}

/// A livello di applicazione (non di libreria) spesso non ti interessa
/// *quale* variante di errore è successa, ma solo propagarla con
/// contesto leggibile da un umano. Qui entra `anyhow`: il suo
/// `anyhow::Result<T>` può contenere QUALSIASI errore che implementi
/// `std::error::Error` (incluso il nostro `ZfsBackupError`), e
/// `.context(...)` permette di aggiungere un messaggio a ogni livello
/// di risalita — costruendo una catena di causa-effetto simile a un
/// traceback Python, ma esplicita e leggibile.
fn load_and_check(path: &Path) -> anyhow::Result<()> {
    use anyhow::Context;

    let content = read_pool_file(path)
        .with_context(|| format!("failed to load pool file {}", path.display()))?;

    let name = content.trim();
    validate_pool_name(name)
        .with_context(|| format!("pool file {} contains an invalid name", path.display()))?;

    println!("pool '{name}' loaded and valid");
    Ok(())
}

fn main() -> anyhow::Result<()> {
    // Caso 1: file inesistente -> errore di IO, convertito e arricchito.
    match load_and_check(Path::new("/nonexistent/pool/file")) {
        Ok(()) => {}
        // `{:#}` stampa l'intera catena di contesto, un livello per
        // riga: "failed to load pool file ... : IO error: ...".
        // È l'equivalente di leggere un traceback Python dal basso
        // (l'eccezione originale) verso l'alto (dove è stata gestita),
        // ma costruito esplicitamente invece che dallo stack unwinding.
        Err(e) => println!("expected failure:\n{e:#}"),
    }

    // Caso 2: nome pool non valido (contiene uno spazio).
    let tmp = std::env::temp_dir().join("step3-demo-pool.txt");
    std::fs::write(&tmp, "tank pool")?;
    match load_and_check(&tmp) {
        Ok(()) => {}
        Err(e) => println!("expected failure:\n{e:#}"),
    }
    std::fs::remove_file(&tmp)?;

    // Caso 3: tutto ok.
    let tmp_ok = std::env::temp_dir().join("step3-demo-pool-ok.txt");
    std::fs::write(&tmp_ok, "tank")?;
    load_and_check(&tmp_ok)?;
    std::fs::remove_file(&tmp_ok)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_name_is_rejected() {
        let err = validate_pool_name("").unwrap_err();
        assert!(matches!(err, ZfsBackupError::ConfigError(_)));
    }

    #[test]
    fn name_with_space_is_rejected() {
        let err = validate_pool_name("tank pool").unwrap_err();
        assert_eq!(
            err.to_string(),
            "Configuration error: pool name 'tank pool' must not contain spaces"
        );
    }

    #[test]
    fn valid_name_passes() {
        assert!(validate_pool_name("tank").is_ok());
    }

    #[test]
    fn missing_file_becomes_io_error_variant() {
        let err = read_pool_file(Path::new("/definitely/not/here")).unwrap_err();
        assert!(matches!(err, ZfsBackupError::IoError(_)));
    }

    #[test]
    fn anyhow_context_is_appended() {
        let err = load_and_check(Path::new("/definitely/not/here")).unwrap_err();
        let message = format!("{err:#}");
        assert!(message.contains("failed to load pool file"));
        assert!(message.contains("IO error"));
    }
}
