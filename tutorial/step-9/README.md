# Step 9 — Tutto insieme, e una mappa Python → Rust

> Obiettivo: un unico programma che carica configurazione, tiene stato
> in SQLite, filtra filesystem con regex, decide l'azione di backup e
> logga con `tracing` — lo scheletro completo del tool reale, meno la
> parte SSH/ZFS vera (vedi step 7 e 8 per il perché). Chiudiamo con una
> tabella riassuntiva di tutti gli idiomi visti negli step precedenti.

## Cosa fare

```bash
cd tutorial/step-9
RUST_LOG=info cargo run
cargo test
```

Nell'output di `cargo run` nota: `tank/docker` non compare mai (escluso
da `skip_filesystems`), `tank/data` è "already up to date", `tank/logs`
richiede un incremento (`@daily-01 => @daily-02`), `tank/new` produce
un `ERROR` — nessuno snapshot in comune, serve inizializzazione
manuale. Sono i tre rami di `BackupDecision` che si vedono anche in
`../../src/backup.rs::backup_filesystem`.

## Cosa c'è di nuovo qui, rispetto agli step precedenti

**Filtrare con regex compilate una volta, poi iterare:**

```rust
let patterns: Vec<Regex> = skip_patterns.iter().map(|p| Regex::new(p)...).collect::<Result<_, _>>()?;
let kept = names.into_iter()
    .filter(|name| !patterns.iter().any(|re| re.is_match(name)))
    .collect();
```

Python:

```python
patterns = [re.compile(p) for p in skip_filesystems]
kept = [name for name in names if not any(p.search(name) for p in patterns)]
```

Stessa forma in entrambi i linguaggi: compila i pattern una sola volta
fuori dal ciclo caldo (ricompilare una regex a ogni confronto sarebbe
inutilmente lento, in entrambi i linguaggi), poi filtra con `any(...)`.
La differenza, come al solito, è che un pattern regex malformato qui
produce `ZfsBackupError::InvalidPattern` — un valore tipizzato che il
chiamante può ispezionare — invece di un'eccezione `re.error` generica.

**Un `enum` per rappresentare "questi sono gli unici esiti possibili":**

```rust
pub enum BackupDecision {
    UpToDate,
    NeedsInitialization,
    Incremental { from: String, to: String },
    NothingToSend,
}
```

A differenza delle stringhe o dei codici numerici che spesso si usano
in Python per rappresentare uno stato a più vie, un `match` su questo
`enum` **deve** gestire tutte e quattro le varianti (o dichiarare
esplicitamente `_ => ...`): se in futuro se ne aggiunge una quinta, il
compilatore indica ogni singolo punto del codice che va aggiornato.

**Logging strutturato con `tracing` invece di `logging`:**

```rust
info!("{fs_name}{from} => {to}");
tracing::error!("Error processing pool {}: {:#}", pool_config.name, e);
```

`tracing::info!`/`error!`/`debug!` funzionano come `logging.info(...)`
di Python, ma con macro verificate a compile-time (un placeholder senza
argomento non compila) e con supporto nativo per "span" annidati
(contesto strutturato che segue automaticamente le chiamate annidate,
utile per correlare i log di un'intera operazione — qui non
approfondito, ma è il motivo per cui `tracing` esiste come crate
separata da un semplice `log`).

## L'intero flusso, in una riga per componente

```
main()                       #[tokio::main], come asyncio.run()      — step 8
  logger::init()              tracing invece di logging               — questo step
  config::load_pool_configs   serde_yaml + scansione directory        — step 4
  process_pool(&pool_config)
    db::initialize_db         schema SQLite, Connection posseduta     — step 7
    (discovery: SSH + zfs)    async fn, tokio::process::Command       — step 8
    backup::filter_filesystems  regex + iteratori                    — questo step
    backup::decide             enum esaustivo                        — questo step
    backup::log_decision       tracing
  errori propagati con anyhow::Result + `?` a ogni livello           — step 3
  tutto costruito su struct, ownership, Option<T>                    — step 2 e 5
```

## Cheat-sheet finale: Python → Rust

| Concetto | Python | Rust | Step |
|---|---|---|---|
| Gestione pacchetti | `pip`/`poetry` + `venv` | `cargo` (integrato) | [1](../step-1/README.md) |
| Type checking | `mypy` (opzionale, esterno) | il compilatore stesso (obbligatorio) | [1](../step-1/README.md) |
| Mutabilità | tutto mutabile per default | `let` immutabile, `let mut` esplicito | [2](../step-2/README.md) |
| Gestione memoria | garbage collector, riferimenti condivisi | ownership: un proprietario, `&`/`&mut` in prestito | [2](../step-2/README.md) |
| Valore assente | `None` universale | `Option<T>`, tipizzato, `match` esaustivo | [2](../step-2/README.md) |
| Errori | eccezioni, `try/except`, non dichiarate nella firma | `Result<T, E>`, dichiarato nel tipo di ritorno, propagato con `?` | [3](../step-3/README.md) |
| Errori di dominio | gerarchia di `class ... (Exception)` | `enum` + `#[derive(thiserror::Error)]` | [3](../step-3/README.md) |
| Errori applicativi + traceback | traceback nativo dell'interprete | `anyhow::Error` + `.context()` | [3](../step-3/README.md) |
| Parsing/validazione dati | `pydantic` o `yaml.safe_load` + validazione manuale | `#[derive(Deserialize)]` (serde), generato a compile-time | [4](../step-4/README.md) |
| Default mutabili (liste) | `field(default_factory=list)` per evitare il default condiviso | `#[serde(default)]` — l'unico comportamento possibile | [4](../step-4/README.md) |
| Interfacce / duck typing | `Protocol`, ABC, o nessun controllo | `trait`, verificato dal compilatore | [5](../step-5/README.md) |
| `__str__` / `__repr__` | due dunder method per convenzione | `Display` / `Debug`, due trait distinti | [5](../step-5/README.md) |
| Dispatch generico | sempre dinamico (duck typing) | statico (`impl Trait`, monomorfizzato) o dinamico (`dyn Trait`), a scelta | [5](../step-5/README.md) |
| Moduli/pacchetti | file + `__init__.py`, tutto importabile | `mod`, con `pub`/`pub(crate)`/privato verificati dal compilatore | [6](../step-6/README.md) |
| Test unitari vs di integrazione | convenzione di naming (`test_*.py`) | confine reale tra crate: unit test vedono i privati, integration test solo il `pub` | [6](../step-6/README.md) |
| Pulizia risorse | `with`/context manager, `try/finally` | `Drop`, automatico alla fine di ogni scope, non serve un blocco dedicato | [4](../step-4/README.md), [7](../step-7/README.md) |
| Database | `sqlite3`, placeholder `?` | `rusqlite`, `params![]`, stessa regola su SQL injection | [7](../step-7/README.md) |
| Riga trovata / non trovata / errore | tutto spesso confuso in `None` | `Result<Option<T>, E>`, tre esiti distinti nel tipo | [7](../step-7/README.md) |
| Codice asincrono | `asyncio`, event loop nativo del linguaggio | `async`/`.await` + runtime esterno (`tokio`) | [8](../step-8/README.md) |
| Task in background | `asyncio.create_task`, cattura per riferimento senza controlli | `tokio::spawn`, richiede dati `'static` posseduti | [8](../step-8/README.md) |
| Logging | modulo `logging` | `tracing`, macro verificate a compile-time | 9 (questo step) |
| "Questi sono gli unici stati possibili" | stringhe, costanti, o `Enum` di `enum.Enum` (senza dati associati) | `enum` con varianti che portano dati, `match` esaustivo | 9 (questo step) |

## Dal tutorial al progetto reale

Ogni step ha citato il file corrispondente in `../../src/`. Per vedere
tutto insieme, per davvero (SSH e ZFS reali inclusi):

```bash
cd ../..              # torna alla radice del repository
cargo build --release
cargo test
```

E leggi, in ordine, `../../src/error.rs`, `config.rs`, `db.rs`,
`ssh.rs`, `backup.rs`, `logger.rs`, `main.rs`, `lib.rs` — a questo
punto dovrebbero risultare familiari riga per riga. `../../INTERNALS.md`
approfondisce l'architettura completa (incluso ciò che qui abbiamo
volutamente semplificato, come il flusso binario di `zfs send`/`recv`
attraverso la pipeline di compressione).

## Cosa NON abbiamo coperto

Per restare un tutorial percorribile in poche ore, abbiamo lasciato
fuori alcuni argomenti che incontrerai leggendo altro codice Rust:

- **Lifetime espliciti** (`fn f<'a>(x: &'a str) -> &'a str`): il
  compilatore li ha sempre inferiti per noi in questo tutorial
  (*lifetime elision*). Servono quando le relazioni tra i prestiti
  diventano ambigue.
- **`unsafe`**: mai usato qui (né nel progetto reale, se non per
  `libc::geteuid()` in `main.rs`, isolato in un singolo blocco
  `unsafe`). È la valvola di sfogo per operazioni che il compilatore
  non può verificare staticamente — da evitare finché non serve
  davvero.
- **`Arc<Mutex<T>>`** e condivisione di stato tra thread/task: non
  serve in questo progetto (ogni pool è processato in sequenza, come
  spiegato nello step 8), ma è il pattern standard quando serve
  davvero condividere dati mutabili tra task concorrenti.
- **Macro dichiarative** (`macro_rules!`): abbiamo *usato* macro
  (`println!`, `derive`, `params!`) ma non ne abbiamo scritte.

Buon proseguimento con Rust.
