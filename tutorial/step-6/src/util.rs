// Questo modulo NON è dichiarato `pub` in `lib.rs` (è `mod util;`, non
// `pub mod util;`): nessun codice esterno alla crate — né `main.rs`,
// né `tests/integration_test.rs` — può scrivere `step6_modules_and_testing::util::...`.
// È un dettaglio implementativo interno.

/// `pub(crate)` rende questa funzione visibile in *tutta* la crate
/// (qualunque modulo sotto `lib.rs`), ma non oltre i suoi confini.
/// Non esiste un equivalente diretto in Python: il closest è la
/// convenzione "modulo interno, non importare da fuori", che però
/// nessuno strumento impedisce di violare. Qui è il compilatore stesso
/// a bloccare l'accesso da `main.rs` o da `tests/`, che sono crate
/// distinte pur facendo parte dello stesso package Cargo.
pub(crate) fn normalize_pool_name(raw: &str) -> String {
    raw.trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_and_lowercases() {
        assert_eq!(normalize_pool_name("  Tank-01 \n"), "tank-01");
    }
}
