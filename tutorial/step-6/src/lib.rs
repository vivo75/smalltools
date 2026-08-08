// Confronta con `../../../src/lib.rs`, quasi identico nella forma:
// pochi `pub mod`, ognuno mappato uno-a-uno su un file. Questa è la
// "root" della libreria: tutto ciò che è `pub` qui è l'API pubblica
// della crate, visibile a `main.rs`, a `tests/`, e (se pubblicata su
// crates.io) a chiunque la aggiunga come dipendenza.

pub mod config;
pub mod error;

// NON `pub`: `util` resta un dettaglio implementativo interno,
// irraggiungibile da fuori la crate (vedi src/util.rs).
mod util;
