// Verbatim (a meno del nome del target `debug` directive) da
// ../../../src/logger.rs.

use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// `tracing` sostituisce il modulo `logging` di Python. Le differenze
/// principali:
///
/// - `tracing::info!("...")` è verificato dal compilatore come
///   qualunque `format!`: un placeholder `{}` senza argomento
///   corrispondente non compila. `logging.info("%s", ...)` di Python
///   scopre un placeholder sbagliato solo quando quella riga di log
///   viene effettivamente eseguita.
/// - `RUST_LOG` (letto da `EnvFilter::try_from_default_env`) è
///   l'equivalente di configurare `logging.basicConfig(level=...)`, ma
///   leggibile da variabile d'ambiente senza codice aggiuntivo, e con
///   una sintassi che permette livelli diversi per modulo (es.
///   `RUST_LOG=step9_full_orchestration=debug,rusqlite=warn`).
pub fn init() {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let fmt_layer = fmt::layer().with_writer(std::io::stderr).with_target(true);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();
}
