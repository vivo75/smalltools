# Step 1 — Il toolchain e i primi passi

> Obiettivo: capire cosa succede quando scrivi `cargo run`, e come i concetti
> base di Rust (variabili, funzioni, `if` come espressione) si confrontano
> con quelli di Python. Nessun codice del tool reale ancora: solo le
> fondamenta.

## Da dove veniamo

Il progetto originale (`../../PROMPT.md`) descrive `st-zfs-send-recv`, un
tool nato come script bash e poi riscritto in Rust (vedi `../../src/`).
Questo tutorial ricostruisce il programma pezzo per pezzo, ma parte da più
indietro: da come si struttura *qualunque* progetto Rust, dando per
scontato che tu sappia già programmare in Python.

## Cosa fare

```bash
cd tutorial/step-1
cargo run
cargo test
cargo fmt --check
cargo clippy
```

## Cargo vs pip/venv/poetry

| Python | Rust | Note |
|---|---|---|
| `venv` + `pip install` | `cargo` | Cargo è **sia** package manager **che** build system, integrato nel linguaggio fin dall'inizio (non un'aggiunta come `pip`/`poetry`). |
| `requirements.txt` / `pyproject.toml` | `Cargo.toml` | Dichiari le dipendenze e le loro versioni qui. |
| (nessun equivalente diretto) | `Cargo.lock` | Il lockfile è generato automaticamente e blocca le versioni esatte di *tutto* l'albero delle dipendenze, non solo di quelle dirette come spesso avviene con `requirements.txt`. |
| `python script.py` | `cargo run` | Compila e poi esegue. La prima volta è più lento (deve compilare), le successive sono rapide grazie alla cache incrementale in `target/`. |
| `black` / `autopep8` | `cargo fmt` | Formattazione automatica, ma è lo *standard* della community, non una scelta tra tante. |
| `flake8` / `pylint` | `cargo clippy` | Linter ufficiale, mantenuto dal team Rust stesso, con centinaia di controlli su idiomi ed errori comuni. |
| `mypy` | (built-in) | In Rust il type-checking non è opzionale né un tool separato: è il compilatore stesso. Non esiste un percorso "non tipizzato". |

Non c'è un `Cargo.toml` a livello di questa cartella condiviso con gli
altri step: ogni `step-N` è una crate Rust indipendente e autosufficiente,
così puoi entrare in una cartella, lanciare `cargo test` e vedere subito
se hai capito il concetto, senza dipendere dagli step successivi.

## Concetti Rust introdotti

### 1. Compilazione statica, non interpretazione

In Python `python script.py` legge ed esegue riga per riga (bytecode
compilato al volo, ma senza verifiche complete a monte). In Rust,
`cargo build`/`cargo run` compilano l'intero programma in un binario
nativo *prima* di eseguirlo. Un errore di tipo, un nome sbagliato, un
riferimento invalido: tutto questo blocca la compilazione, non esplode a
runtime tre mesi dopo in produzione su un ramo di codice raramente
eseguito.

### 2. `let` è immutabile per default

```rust
let config_dir = Path::new("/etc/st-zfs-send-recv.d"); // immutabile
let mut counter = 0;                                    // esplicitamente mutabile
```

In Python ogni variabile è "mutabile" nel senso che può essere
riassegnata in qualsiasi momento. In Rust il default è l'opposto:
un binding `let` non può essere riassegnato a meno di scrivere
esplicitamente `let mut`. Questo non è pedanteria: rende immediatamente
visibile, leggendo la firma di una funzione o una dichiarazione, quali
dati possono cambiare e quali no — un'informazione che in Python devi
dedurre leggendo tutto il corpo della funzione.

### 3. Tipi espliciti nelle firme delle funzioni

```rust
fn config_dir_exists(dir: &Path) -> bool {
```

Equivalente Python con type hints:

```python
def config_dir_exists(dir: Path) -> bool:
```

La differenza cruciale: in Python i type hint sono **documentazione**,
verificata solo se esegui `mypy` separatamente, e comunque non impediscono
l'esecuzione di codice con tipi sbagliati. In Rust sono **contratti
verificati dal compilatore**: se provi a passare un `i32` dove serve un
`&Path`, il programma non compila. Punto.

### 4. `&` è un prestito (borrow), non un puntatore generico

`&Path` non è "un riferimento a un oggetto" nel senso vago di Python
(dove *tutto* è passato per riferimento a un oggetto sull'heap gestito dal
GC). È un prestito **temporaneo e verificato staticamente**: il
compilatore garantisce che quel riferimento non sopravviva al dato a cui
punta, e che non esistano contemporaneamente un prestito mutabile e uno
immutabile sullo stesso dato. Approfondiremo ownership e borrowing nello
[step 2](../step-2/README.md) — qui basta riconoscere la sintassi.

### 5. `if` è un'espressione

```rust
let exit_code = if config_dir_exists(config_dir) { 0 } else { 1 };
```

In Python useresti l'operatore ternario `0 if cond else 1`, utilizzabile
solo per espressioni semplici su una riga. In Rust `if`/`else` è
un'espressione **generale**: ogni ramo può contenere blocchi di codice
arbitrariamente complessi, purché entrambi i rami producano lo stesso
tipo. Niente `return` sparsi per simulare un valore condizionale.

### 6. Test dentro al file sorgente

```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_dir_is_false() { ... }
}
```

Python separa tipicamente i test in `test_*.py` scoperti da `pytest` per
convenzione di naming. Rust integra i test **nello stesso file** del
codice che testano (per unit test) grazie all'attributo `#[cfg(test)]`,
che dice al compilatore "includi questo modulo solo quando compili per
`cargo test`". Il codice di test non finisce mai nel binario di release:
non è una questione di disciplina del programmatore, è imposto dal
compilatore. Vedremo l'organizzazione dei test più in dettaglio nello
[step 6](../step-6/README.md).

## Perché queste scelte nel progetto reale

Il vero `st-zfs-send-recv` (`../../src/main.rs`) usa esattamente questo
schema: un `banner` iniziale via `tracing::info!` (che vedremo nello
step 9), verifiche di precondizioni (`/etc/st-zfs-send-recv.d` esiste?
sei root?) prima di fare qualunque lavoro, ed exit code espliciti tramite
`anyhow::bail!` che risale fino a `main() -> Result<()>`. L'idea di
"fallire presto e in modo esplicito" — tipica di Rust — è la stessa
filosofia dietro a tutto il tool: meglio un errore chiaro all'avvio che
un backup ZFS a metà.

## Prossimo passo

[Step 2 — Ownership, struct e Option](../step-2/README.md): modelliamo
`PoolConfig`, la configurazione di un pool ZFS, come una struct Rust, e
vediamo cosa significa davvero "possedere" un dato invece di condividerlo
tramite un garbage collector.
