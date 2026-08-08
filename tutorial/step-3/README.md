# Step 3 — Niente eccezioni: `Result`, `?`, `thiserror`, `anyhow`

> Obiettivo: capire come Rust gestisce i fallimenti senza eccezioni, e
> perché il progetto reale usa **due** strategie di errore diverse
> insieme (`thiserror` ed `anyhow`) invece di sceglierne una sola.

## Cosa fare

```bash
cd tutorial/step-3
cargo run
cargo test
```

Guarda l'output di `cargo run`: tre scenari (file mancante, nome pool
non valido, tutto ok), e per i primi due la catena di contesto stampata
da `{:#}`.

## Il cambio di paradigma

Python:

```python
def load_and_check(path: Path) -> None:
    try:
        content = path.read_text()
    except OSError as e:
        raise RuntimeError(f"failed to load pool file {path}") from e

    name = content.strip()
    if not name:
        raise ValueError(f"pool file {path} contains an invalid name")
```

Un errore può risalire lo stack passando per funzioni che non lo
menzionano affatto nella loro firma. Devi leggere il *corpo* di ogni
funzione (o la documentazione, se esiste ed è aggiornata) per sapere
cosa può sollevare.

Rust:

```rust
fn load_and_check(path: &Path) -> anyhow::Result<()> {
    let content = read_pool_file(path)
        .with_context(|| format!("failed to load pool file {}", path.display()))?;
    let name = content.trim();
    validate_pool_name(name)
        .with_context(|| format!("pool file {} contains an invalid name", path.display()))?;
    Ok(())
}
```

La possibilità di fallire è **parte del tipo di ritorno**:
`anyhow::Result<()>`. Non puoi ignorarla per sbaglio: se chiami una
funzione che restituisce `Result` e non fai nulla con il risultato, il
compilatore emette un warning (`#[must_use]` su `Result`). Se vuoi
propagare l'errore al chiamante, `?` lo fa in una riga.

## `Result<T, E>` non è `Optional` più un'eccezione

```rust
enum Result<T, E> {
    Ok(T),
    Err(E),
}
```

È un `enum` con due varianti, esattamente come `Option<T>` ne ha due
(`Some`/`None`) visto nello [step 2](../step-2/README.md). Non c'è
nessuna magia: gestisci un `Result` con `match`, con `?`, o con i metodi
di comodo (`.unwrap()`, `.expect(msg)`, `.unwrap_or(default)`,
`.is_ok()`...). Il compilatore *non ti lascia dimenticare* di gestirlo:
un `Result` che finisce silenziosamente ignorato produce un warning.

## `?`: propagazione, non magia

```rust
let content = std::fs::read_to_string(path)?;
```

è zucchero sintattico per:

```rust
let content = match std::fs::read_to_string(path) {
    Ok(v) => v,
    Err(e) => return Err(From::from(e)),
};
```

Tre cose in una riga: unwrap del successo, conversione automatica del
tipo di errore (tramite `From`, vedi sotto), `return` anticipato in
caso di fallimento. Ricorda un `except ... as e: raise NewError() from
e` ripetuto ad ogni chiamata — ma qui il compilatore lo genera per te, e
non puoi dimenticartelo perché senza `?` (o un `match` esplicito) il
codice semplicemente non compila (i tipi non combaciano: `Result<T, E>`
non è `T`).

## `thiserror`: errori tipizzati "a la libreria"

`src/error.rs` in questo step:

```rust
#[derive(Error, Debug)]
pub enum ZfsBackupError {
    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("IO error: {0}")]
    IoError(#[from] io::Error),
}
```

`#[derive(Error)]` (dalla crate `thiserror`) genera l'implementazione di
`std::error::Error` e di `Display` a partire dalle stringhe `#[error("...")]`
— l'equivalente di scrivere a mano `__str__` per ogni sottoclasse di
`Exception` in Python, ma generato dal compilatore invece che da te.

`#[from]` genera `impl From<io::Error> for ZfsBackupError`, che è
esattamente ciò che rende possibile l'operatore `?` dentro
`read_pool_file`: converte automaticamente l'errore di IO nella
variante giusta del nostro enum. In Python il corrispondente sarebbe
scrivere ripetutamente `except OSError as e: raise ZfsBackupError(...)
from e` — qui basta annotare il campo una volta con `#[from]`.

Usa `thiserror` quando **il chiamante potrebbe voler distinguere** tra i
tipi di errore (es. "è un errore di rete? riprova. È un errore di
configurazione? fermati e avvisa l'utente."). È pensato per essere il
tipo di errore pubblico di una libreria o di un modulo interno ben
definito.

## `anyhow`: errori opachi "a la applicazione"

```rust
fn load_and_check(path: &Path) -> anyhow::Result<()> {
```

`anyhow::Result<T>` è `Result<T, anyhow::Error>`, dove `anyhow::Error`
può avvolgere **qualunque** tipo che implementi `std::error::Error`
(incluso il nostro `ZfsBackupError`, un `io::Error`, un errore di
`serde_yaml`, ecc.) in un unico contenitore. `.context(...)` /
`.with_context(...)` aggiungono un messaggio leggibile a ogni livello
di risalita, costruendo una catena — un po' come un traceback Python,
ma costruita esplicitamente e stampabile con `{:#}` (multi-riga) o `{}`
(solo il messaggio più esterno).

Usa `anyhow` a **livello applicativo**, dove non ti interessa più fare
`match` fine sul tipo di errore: ti interessa solo loggarlo con
contesto chiaro e uscire con un codice di errore diverso da zero — è
esattamente quello che fa `main()` nel vero
`../../src/main.rs`:

```rust
match process_pool(&pool_config).await {
    Ok(_) => info!("Successfully processed pool: {}", pool_config.name),
    Err(e) => tracing::error!("Error processing pool {}: {:#}", pool_config.name, e),
}
```

## Perché entrambe nel progetto reale

- `error.rs` definisce `ZfsBackupError` con `thiserror`: è il
  vocabolario di errori "di dominio" del tool (SSH fallito, comando ZFS
  fallito, snapshot in stato invalido...), pensato per essere
  riconoscibile e, in futuro, testabile con `matches!` come facciamo
  nei test qui sopra.
- Quasi tutto il resto del codice (`config.rs`, `backup.rs`, `main.rs`)
  usa `anyhow::Result` con `.context()`, perché a quel livello interessa
  solo propagare un errore leggibile fino a `main()`, che lo logga e
  passa al pool successivo (vedi `main.rs`: un pool che fallisce non
  blocca gli altri).

Questo mix — tipi di errore precisi alle fondamenta, contesto opaco ma
ricco più in alto — è l'idioma standard nell'ecosistema Rust applicativo
(diverso da una libreria pubblicata su crates.io, dove di solito si usa
*solo* `thiserror`, per non forzare agli utenti una dipendenza da
`anyhow`).

## Prossimo passo

[Step 4 — Serde: deserializzare la configurazione YAML](../step-4/README.md):
finora `PoolConfig` viene costruito a mano. Ora lo leggiamo da file YAML
reali, con `serde`, e vediamo come sostituisce `yaml.safe_load()` +
validazione manuale con `#[derive(Deserialize)]`.
