# Step 8 — Concorrenza con `async`/`.await` e `tokio`

> Obiettivo: capire `async fn`, `.await`, `#[tokio::main]`, e perché
> `tokio::spawn` impone vincoli di ownership più stretti di una normale
> chiamata di funzione — confrontando ogni pezzo con `asyncio` di
> Python.

## Cosa fare

```bash
cd tutorial/step-8
cargo run
cargo test
```

Guarda i tempi stampati da `cargo run`: la versione sequenziale di tre
"scoperte remote" (ciascuna con 150ms di latenza simulata) impiega
~450ms; la versione concorrente, che le lancia tutte insieme, ~150ms.
`cargo test` include un test (`concurrent_discovery_is_faster_than_sequential`)
che verifica automaticamente questa proprietà, non solo la mostra a
schermo.

## Perché non abbiamo un vero host SSH

Il vero `../../src/ssh.rs` apre una sessione SSH con `openssh::Session`
ed esegue `zfs list`/`zfs send` sul host remoto. In questo ambiente non
c'è un server SSH né un pool ZFS disponibili, quindi simuliamo la
stessa *forma* di codice con comandi locali (`echo`, `cat`) e una
`tokio::time::sleep` artificiale per imitare la latenza di rete. La
lezione su `async`/ownership/concorrenza è identica; cambia solo cosa
c'è dall'altra parte della chiamata.

## `async fn` non esegue nulla finché non fai `.await`

```rust
async fn discover_filesystems(pool: String) -> Result<Vec<String>> { ... }

let future = discover_filesystems("tank".to_string()); // NON esegue ancora nulla
let result = future.await; // ora, e solo ora, parte
```

Equivalente Python:

```python
async def discover_filesystems(pool: str) -> list[str]: ...

coro = discover_filesystems("tank")  # non esegue ancora nulla
result = await coro                   # ora parte
```

Concettualmente identico: sia `async fn` in Rust sia `async def` in
Python producono un valore "lavoro non ancora iniziato" (`Future` /
coroutine) quando chiamati, e serve `.await`/`await` per farlo
avanzare. La differenza sta in cosa succede *dietro le quinte*:

## Niente event loop globale implicito

Python ha un event loop che gira "da qualche parte", gestito da
`asyncio.run()`. Rust non ha un runtime async incorporato nel
linguaggio: `async`/`.await` sono solo sintassi che il compilatore
trasforma in una macchina a stati; **serve una crate esterna** (qui
`tokio`, l'altra scelta comune è `async-std`) per fornire l'effettivo
runtime che esegue quella macchina a stati. `#[tokio::main]` è una
macro che trasforma:

```rust
#[tokio::main]
async fn main() -> Result<()> { ... }
```

in qualcosa di equivalente a:

```rust
fn main() -> Result<()> {
    tokio::runtime::Runtime::new().unwrap().block_on(async { ... })
}
```

— lo stesso ruolo di `asyncio.run(main())` in Python, ma esplicito
tramite un attributo invece che una chiamata di funzione nascosta in
fondo al file.

## `tokio::spawn` e il vincolo `'static`

```rust
let handles: Vec<_> = pools
    .iter()
    .map(|pool| tokio::spawn(discover_filesystems(pool.clone())))
    .collect();
```

Nota `pool.clone()`, non `pool`. `tokio::spawn` pianifica un task che
può essere eseguito dal runtime **indipendentemente** dallo scope che
lo ha creato — potenzialmente su un altro thread, e potenzialmente
sopravvivendo più a lungo del blocco di codice corrente. Per questo
Rust impone che la future passata a `spawn` sia `'static`: non può
contenere riferimenti in prestito (`&pool`) che potrebbero diventare
invalidi. Deve possedere i propri dati.

Questo è un vincolo che **Python non ha**, ed è anche uno dei motivi
per cui `asyncio.create_task()` può catturare per riferimento variabili
del closure senza alcun controllo — se quella variabile viene
modificata o distrutta altrove mentre il task gira, in Python è un bug
silenzioso a runtime (o un `RuntimeError` a scoperta tardiva); in Rust
è un errore di compilazione immediato, con un messaggio che indica
esattamente quale prestito non sopravvive abbastanza a lungo.

## Ownership che attraversa un `.await`

```rust
let mut stdin = child.stdin.take().context("child has no stdin")?;
...
let writer = tokio::spawn(async move {
    stdin.write_all(&payload_owned).await?;
    drop(stdin);
    Ok::<(), anyhow::Error>(())
});
```

`async move` sposta l'ownership di `stdin` e `payload_owned` **dentro**
il blocco async. Da quel momento, il task che scrive sullo stdin del
processo figlio possiede quelle risorse in modo esclusivo — nessun'altra
parte del programma può interferire con `stdin` mentre questo task gira.
`drop(stdin)` chiude esplicitamente lo stdin per segnalare EOF al
processo figlio: in Python l'equivalente sarebbe chiudere il file o
usare `proc.stdin.close()` — ma qui la chiusura è anche un'operazione
di ownership: dopo `drop(stdin)`, quel valore non esiste più, punto,
non solo "è stato chiuso ma la variabile esiste ancora e potresti
provare a riusarla per errore".

## Scrivi e leggi concorrentemente: evitare il deadlock delle pipe

```rust
let writer = tokio::spawn(async move { stdin.write_all(...).await; ... });
stdout.read_to_end(&mut collected).await?;
writer.await??;
```

Se scrivessimo *tutto* lo stdin prima di iniziare a leggere lo stdout,
con un payload abbastanza grande il processo figlio si bloccherebbe
(il suo buffer di output si riempie, nessuno lo sta svuotando, e il
figlio smette di leggere altro stdin) — un classico deadlock da pipe.
La soluzione è la stessa in Rust e in Python: scrivere e leggere
**concorrentemente**. In Python tipicamente con
`asyncio.gather(write_task, read_task)` o con
`subprocess.communicate()` (che lo fa già internamente con i thread);
qui con `tokio::spawn` per lo scrittore e `.await` diretto per il
lettore nello stesso task chiamante. È esattamente il pattern che
`../../src/backup.rs::perform_incremental_backup` userebbe in una
versione più robusta (il codice attuale scrive tutto lo stdin prima di
attendere l'uscita — accettabile perché gli stream ZFS reali passano
attraverso `Stdio::piped()` con buffer gestiti dal kernel, ma il
principio generale resta questo).

## `?` due volte: `handle.await??`

```rust
concurrent_results.push(handle.await??);
```

Due diversi tipi di fallimento, entrambi propagati con `?`:

1. `handle.await` restituisce `Result<T, JoinError>` — `JoinError`
   accade se il task è andato in panic o è stato cancellato. Primo `?`.
2. `T` qui è a sua volta `Result<Vec<String>, anyhow::Error>` (il
   ritorno di `discover_filesystems`). Secondo `?`.

Non c'è un equivalente diretto in Python: un `asyncio.Task` che va in
eccezione la ripropaga direttamente su `await task`, un solo livello.
Qui i due livelli — "il task è crashato" vs "il task ha restituito un
errore applicativo" — restano tipati separatamente, e il compilatore ti
costringe a gestirli entrambi (anche solo con `?`).

## Perché queste scelte nel progetto reale

`../../src/main.rs` usa `#[tokio::main]` e processa i pool **in
sequenza** (un `for pool_config in configs { process_pool(...).await }`),
non concorrentemente. È una scelta deliberata, non un'occasione persa:
backup multipli in parallelo verso lo stesso host remoto competerebbero
per la stessa banda di rete e lo stesso storage ZFS, quindi il
parallelismo qui non porterebbe benefici reali e aggiungerebbe solo
complessità (log intrecciati tra pool diversi, più difficile da
diagnosticare quando un backup fallisce). Il file `PROMPT.md` elenca
esplicitamente "parallel pool processing" tra i "Optional Enhancements
(Future)" — è una scelta rimandata consapevolmente, non un limite del
linguaggio: il pattern mostrato in questo step è esattamente ciò che
serve, se in futuro si decidesse di introdurlo.

## Prossimo passo

[Step 9 — Orchestrazione completa e cheat-sheet finale](../step-9/README.md):
mettiamo insieme configurazione, database, errori e (concettualmente)
SSH in un unico flusso, con `tracing` per il logging strutturato, e
chiudiamo con una tabella riassuntiva Python → Rust.
