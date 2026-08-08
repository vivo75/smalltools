# Da Python a Rust, tramite `st-zfs-send-recv`

Questo tutorial insegna Rust a un programmatore Python esperto,
ricostruendo passo per passo il tool reale di questo repository:
`st-zfs-send-recv`, un backup incrementale ZFS via SSH (la specifica
completa è in [`../PROMPT.md`](../PROMPT.md), l'implementazione finita
in [`../src/`](../src/)).

Non è un elenco astratto di feature del linguaggio: ogni step risolve
un pezzo concreto del problema — modellare una configurazione, gestire
un errore di rete, tenere traccia di snapshot in SQLite, parlare con un
host remoto in modo asincrono — e ogni README spiega **perché** il
codice reale è scritto come è scritto, confrontandolo riga per riga con
l'equivalente Python più naturale.

## Come è organizzato

Ogni `step-N/` è una crate Cargo **indipendente e autocompilabile**:

```bash
cd step-N
cargo run     # dove ha senso
cargo test
```

Non serve seguire l'ordine in una singola sessione: ogni step ha un
README autonomo con codice funzionante e testato. Detto questo, sono
pensati per essere letti in sequenza — ogni step introduce un concetto
e lo dà per acquisito in quelli successivi.

| Step | Argomento | Concetti Rust chiave | Corrisponde a |
|---|---|---|---|
| [1](step-1/README.md) | Il toolchain | `cargo`, compilazione statica, `let`/`let mut`, `if` come espressione | — |
| [2](step-2/README.md) | Ownership e struct | ownership, borrow (`&`/`&mut`), `Option<T>`, `struct` | `src/config.rs` (forma dati) |
| [3](step-3/README.md) | Gestione errori | `Result<T, E>`, `?`, `thiserror`, `anyhow` | `src/error.rs` |
| [4](step-4/README.md) | Serde | `#[derive(Deserialize)]`, `#[serde(default)]`, macro procedurali | `src/config.rs` |
| [5](step-5/README.md) | Trait e generics | `trait`, `impl Trait` (statico), `dyn Trait` (dinamico), `FromStr`, `Display` | (pattern applicabile a `src/backup.rs`) |
| [6](step-6/README.md) | Moduli e test | `mod`, `pub`/`pub(crate)`, unit test vs integration test | `src/lib.rs`, `tests/` |
| [7](step-7/README.md) | SQLite | `rusqlite`, `Connection` (RAII), `params!`, `Option<Result<T>>` | `src/db.rs` |
| [8](step-8/README.md) | Async/await | `async fn`, `.await`, `tokio::spawn`, vincolo `'static` | `src/ssh.rs` |
| [9](step-9/README.md) | Tutto insieme | orchestrazione, `tracing`, cheat-sheet Python → Rust | `src/backup.rs`, `src/main.rs` |

## Perché questo ordine

Rispecchia grosso modo la dipendenza logica dei moduli reali: prima i
dati (`PoolConfig`, step 2), poi come possono fallire (step 3), poi
come arrivano da disco (step 4). Trait e generics (step 5) sono un
interludio deliberato — non servono a caso, ma è il momento in cui hai
visto abbastanza codice reale (struct, `Result`, `derive`) da apprezzare
cosa sta succedendo dietro quei `#[derive(...)]` usati fin dallo step
2. Poi moduli/test (step 6), persistenza (step 7) e concorrenza (step
8) — le tre parti più "infrastrutturali" del tool — e infine
l'orchestrazione completa (step 9), che le mette tutte insieme.

## Prerequisiti

- Un programmatore che conosce già Python bene (tipi, funzioni,
  eccezioni, moduli) e vuole imparare Rust confrontando ogni concetto
  con qualcosa che già conosce.
- Rust e Cargo installati (`rustc --version`, `cargo --version`).
  Nessuna conoscenza pregressa di Rust è richiesta.
- Connessione di rete per scaricare le dipendenze la prima volta che
  lanci `cargo build`/`cargo test` in ciascuno step (Cargo le mette
  poi in cache in `~/.cargo`).

## Cosa NON serve per seguire il tutorial

A differenza del tool reale, nessuno step richiede un pool ZFS, un
host SSH raggiungibile, o privilegi di root: dove il codice reale
parla con `zfs`/`ssh`, gli step 7 e 8 usano rispettivamente un
database SQLite in memoria e comandi locali (`echo`, `cat`) come
sostituti, spiegando esplicitamente nel README cosa cambierebbe in
produzione.

## Dopo il tutorial

Il [cheat-sheet finale nello step 9](step-9/README.md#cheat-sheet-finale-python--rust)
riassume tutti gli idiomi Python → Rust visti. Da lì, il passo naturale
è leggere il codice reale in [`../src/`](../src/) e la documentazione
di progetto: [`../INTERNALS.md`](../INTERNALS.md) (architettura e
data flow) e [`../DEVELOPMENT.md`](../DEVELOPMENT.md) (workflow di
sviluppo, strumenti di qualità del codice).
