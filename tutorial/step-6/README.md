# Step 6 — Moduli, visibilità e i tre livelli di test

> Obiettivo: capire come Rust organizza codice su più file (`mod`,
> `pub`, `pub(crate)`), il pattern "lib + bin" usato dal progetto reale,
> e perché `cargo test` esegue tre cose diverse quando lanci un solo
> comando.

## Cosa fare

```bash
cd tutorial/step-6
cargo run
cargo test
```

Guarda l'output di `cargo test`: **quattro** sezioni separate
("Running unittests src/lib.rs", "Running unittests src/main.rs",
"Running tests/integration_test.rs", "Doc-tests"). Non è rumore: sono
tre meccanismi di test concettualmente diversi, spiegati sotto.

## Il layout del progetto

```
step-6/
├── Cargo.toml
├── src/
│   ├── lib.rs      # radice della libreria: dichiara i moduli pubblici
│   ├── config.rs   # modulo `config`
│   ├── error.rs    # modulo `error`
│   ├── util.rs      # modulo `util`, privato alla crate
│   └── main.rs     # binario: usa la libreria come farebbe un esterno
└── tests/
    └── integration_test.rs   # crate di test separata
```

Confronta con `../../src/`: stesso schema esatto (`lib.rs`, `config.rs`,
`error.rs`, `db.rs`, `ssh.rs`, `backup.rs`, `main.rs`, più
`tests/integration_test.rs`). Non è un caso: è il layout standard di
un progetto Rust binario non banale, tanto quanto un package Python con
`__init__.py` + moduli + `tests/` è lo standard per un progetto Python
non banale.

## `mod` vs pacchetti Python

```python
# my_package/__init__.py
from . import config
from . import error
```

```rust
// src/lib.rs
pub mod config;
pub mod error;
mod util;
```

`pub mod config;` dice a Rust "esiste un file `src/config.rs` (o
`src/config/mod.rs`), trattalo come sotto-modulo chiamato `config`, e
rendilo visibile a chi usa questa crate". `mod util;` (senza `pub`) fa
la stessa cosa ma **non lo espone all'esterno**.

La differenza pratica più importante rispetto a Python: gli `import` di
Python funzionano indipendentemente da dove il codice chiamante si
trova rispetto al modulo (con l'eccezione degli underscore prefix, che
sono solo convenzione). In Rust, la visibilità è **verificata dal
compilatore** in base a dove il modulo è dichiarato e con quale
livello di `pub`:

| Rust | Visibile da | Equivalente Python (informale) |
|---|---|---|
| (niente, default) | solo nel modulo stesso e nei suoi figli | `_nome` — ma è solo convenzione |
| `pub(crate)` | ovunque nella stessa crate | nessun equivalente reale |
| `pub` | ovunque, anche fuori dalla crate | nome pubblico, esportato |

In Python non c'è modo di dire "visibile a tutto il mio pacchetto, ma
non a chi lo importa dall'esterno" — `pub(crate)` in `src/util.rs` di
questo step è esattamente questo, e il compilatore lo fa rispettare
(`config.rs` può chiamare `util::normalize_pool_name`, ma `main.rs` e
`tests/integration_test.rs`, essendo crate diverse, non possono — prova
a decommentare le righe segnate in entrambi i file per vederlo con i
tuoi occhi).

## Il pattern "lib + bin"

```toml
[lib]
name = "step6_modules_and_testing"
path = "src/lib.rs"

[[bin]]
name = "step6-modules-and-testing"
path = "src/main.rs"
```

Un package Cargo può produrre **sia** una libreria **sia** un
eseguibile dallo stesso codice sorgente. `main.rs` diventa
volutamente sottile: analizza gli argomenti, chiama `logger::init()`,
chiama le funzioni della libreria, gestisce l'exit code — tutta la
logica vera vive in moduli testabili dentro `lib.rs`.

Perché non mettere tutto in `main.rs` e basta, come spesso si fa in un
piccolo script Python? Perché **solo il codice raggiungibile da
`lib.rs` è testabile da `tests/`** (che collega la libreria come
dipendenza esterna, non il binario). Se la logica di business vivesse
solo in `main.rs`, gli integration test non potrebbero toccarla
affatto. È l'equivalente Python di tenere la logica in funzioni
importabili da `mypackage/core.py` invece che infilata dentro
`if __name__ == "__main__":` — una disciplina che in Python è solo
buona pratica facoltativa, qui è imposta dalla struttura stessa del
progetto.

## I tre livelli di test che hai visto in `cargo test`

1. **Unit test** (`#[cfg(test)] mod tests { ... }` dentro `config.rs` e
   `util.rs`): compilati **come parte della stessa crate** del codice
   che testano. Per questo `can_call_pub_crate_helper_directly_from_unit_test`
   può chiamare `util::normalize_pool_name`, una funzione `pub(crate)`
   invisibile dall'esterno. Sono il posto giusto per testare dettagli
   implementativi interni — l'equivalente di test che, in Python,
   importano e chiamano direttamente una funzione `_helper()`
   "privata" di un modulo.

2. **Integration test** (`tests/integration_test.rs`): ognuno di questi
   file è compilato da Cargo come **crate binaria indipendente**, che
   collega `step6_modules_and_testing` come una libreria esterna
   qualunque. Vedono solo l'API `pub`. Sono il posto giusto per
   verificare che l'API pubblica si comporti come promesso — l'analogo
   Rust di un test `pytest` che importa solo `from mypackage import
   PublicClass` e non tocca moduli interni con `_` davanti.

3. **Doc-test** (l'ultima sezione, "Doc-tests ..."): esempi di codice
   scritti nei commenti di documentazione (`/// ```rust ... ````)
   vengono estratti, compilati ed eseguiti come test veri. In questo
   step non ne abbiamo scritti (0 eseguiti), ma è un meccanismo che
   vale la pena conoscere: garantisce che gli esempi nella
   documentazione non diventino obsoleti col tempo, cosa che in Python
   richiederebbe uno strumento esterno come `doctest` (che esiste, ma
   è usato molto meno spesso di quanto lo sia il suo equivalente in
   Rust).

## Perché queste scelte nel progetto reale

`../../src/lib.rs` esiste con l'unico scopo — si veda il commento
originale nel file — di "permettere al binario e ai test di importare i
moduli". `../../tests/integration_test.rs` costruisce `PoolConfig`
importandolo da `st_zfs_send_recv::config`, esattamente come facciamo
qui: verifica che la configurazione, l'unico modulo del progetto reale
senza I/O esterno (rete, filesystem privilegiato, ZFS), si comporti
correttamente attraverso la sua API pubblica.

## Prossimo passo

[Step 7 — SQLite con `rusqlite`](../step-7/README.md): usiamo la
persistenza per tracciare filesystem e snapshot, come fa
`../../src/db.rs`. Vediamo prepared statement, il tipo `Connection`
(che possiede la risorsa OS sottostante), e come `Option`/`Result` si
combinano per rappresentare "riga trovata / non trovata / errore".
