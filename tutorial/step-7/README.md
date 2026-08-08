# Step 7 — Stato persistente con `rusqlite`

> Obiettivo: usare SQLite per tracciare filesystem e snapshot, come fa
> `../../src/db.rs`, e capire come `Option`/`Result` rappresentano in
> modo pulito "riga trovata", "nessuna riga" ed "errore" — tre esiti
> che in Python spesso si confondono dietro allo stesso `None`.

## Cosa fare

```bash
cd tutorial/step-7
cargo run
cargo test
```

## Il problema di dominio

Per decidere se e come fare un backup incrementale, il tool deve
rispondere a due domande per ogni filesystem:

1. Qual è lo snapshot più recente presente **sia** sulla sorgente
   **sia** sulla destinazione? (il punto di partenza dell'incremento)
2. Qual è lo snapshot più recente sulla sorgente? (il punto di arrivo)

`get_common_snapshot` e `get_remote_newest_snapshot` in `src/db.rs`
rispondono esattamente a queste domande con due query SQL, verbatim
identiche a quelle del progetto reale.

## `Connection` come risorsa posseduta (RAII)

```rust
let conn = Connection::open_in_memory()?;
```

`Connection` non è "un handle che devi ricordarti di chiudere", è un
valore Rust normale: possiede il file descriptor sqlite, e il suo
`Drop` lo chiude quando esce di scope. Stesso principio di `TempDir`
(step 4) e `File` in generale. Confronta con Python:

```python
conn = sqlite3.connect(":memory:")
try:
    ...
finally:
    conn.close()
```

oppure, meglio, con il context manager. In Rust il "meglio" è
*sempre* il default: non serve scegliere di usare un `with`, perché
non esiste un modo di ottenere una `Connection` senza la garanzia di
chiusura automatica.

## `Option<String>`: "nessuna riga" non è un errore

```rust
conn.query_row(query, params![pool, fs_name], |row| row.get::<_, String>(0))
    .optional()
    .context("Failed to query common snapshot")
```

Tre livelli sovrapposti, ognuno con un significato preciso:

- `Err(...)` — qualcosa è andato storto (connessione persa, SQL
  malformato, tipo di colonna sbagliato). Un vero errore.
- `Ok(None)` — la query ha funzionato, semplicemente non ci sono righe.
  Non è un errore: è un esito legittimo e previsto ("non c'è ancora
  nessuno snapshot in comune, serve un'inizializzazione manuale").
- `Ok(Some(valore))` — la query ha funzionato e ha trovato la riga.

In Python con `sqlite3`, `cursor.fetchone()` restituisce `None` sia per
"nessuna riga" sia — se non stai attento — può essere confuso con un
valore di colonna che è `None` (`NULL` SQL). Qui i due concetti sono
tenuti separati per costruzione: `Option<String>` esterno significa
"riga assente", un eventuale `NULL` dentro una colonna sarebbe un
secondo `Option` annidato (`Option<Option<String>>`), esplicito e
impossibile da confondere con il primo.

Senza `.optional()`, `query_row` restituirebbe
`Err(rusqlite::Error::QueryReturnedNoRows)` quando non trova righe —
tecnicamente un errore Rust, ma semanticamente non lo è per il nostro
dominio. `.optional()` è un adattatore che sposta quel caso specifico
da `Result::Err` a `Result::Ok(None)`: guarda la sua implementazione
concettuale, è solo un `match` che intercetta quella variante
specifica e la trasforma — lo stesso genere di `match` esplicito su cui
insistiamo fin dallo step 2.

## `params!`: valori parametrizzati, mai stringhe concatenate

```rust
conn.execute(&query, params![fs.pool, fs.guid, fs.name])
```

Identico, concettualmente, ai placeholder `?` del modulo `sqlite3` di
Python: i *valori* passano sempre per un canale separato dalla stringa
SQL, che li tratta come dati e non come sintassi — la difesa standard
contro la SQL injection, in qualunque linguaggio.

Nota il contrasto con questa riga, poco sopra nello stesso file:

```rust
let query = format!("INSERT INTO {table} (pool, guid, name) VALUES (?, ?, ?)");
```

Qui *interpoliamo* `table` direttamente nella stringa SQL con
`format!`. Non è un'incoerenza: SQL non permette di parametrizzare i
**nomi delle tabelle** con `?` (solo i valori), quindi l'interpolazione
è l'unico modo. La sicurezza qui non viene dalla sintassi ma dalla
provenienza del dato: `table` nel nostro codice vale sempre una
costante scelta dal programma (`"fs_src"`, `"snap_dst"`, ...), mai un
valore letto da rete o da un file di configurazione controllato da
terzi. Vale la stessa identica regola in Python: `f"SELECT * FROM
{table}"` è sicuro solo se `table` non è mai input esterno.

## Iteratori sul database: `query_map` + `collect`

```rust
let names: Result<Vec<String>, rusqlite::Error> = stmt
    .query_map(params![pool], |row| row.get::<_, String>(0))?
    .collect();
```

`query_map` restituisce un iteratore **pigro**: le righe vengono lette
dal database una alla volta man mano che l'iteratore avanza, non tutte
insieme come farebbe `cursor.fetchall()` in Python. `.collect()` nel
tipo `Result<Vec<String>, rusqlite::Error>` le raccoglie tutte
fermandosi al primo errore — lo stesso idioma "iteratore di `Result` →
`Result` di collezione" visto per il parsing testuale nello
[step 5](../step-5/README.md), applicato qui a righe SQL.

## Perché una tabella `fs_src`/`fs_dst` per lato invece di una colonna `side`

Lo schema (identico al reale) usa quattro tabelle (`fs_src`, `fs_dst`,
`snap_src`, `snap_dst`) invece di due tabelle con una colonna
`side TEXT`. È una scelta di modellazione, non imposta dal linguaggio:
tenere sorgente e destinazione in tabelle separate rende impossibile,
per costruzione, una query che confonda i due lati per errore (non
serve mai un `WHERE side = 'src'` che potresti dimenticare). Il
`JOIN` tra `snap_src` e `snap_dst` sullo stesso `guid` in
`get_common_snapshot` è il cuore dell'algoritmo di backup incrementale:
un `guid` uguale su entrambi i lati significa "è lo stesso identico
snapshot ZFS", perché ZFS assegna i guid in modo che sopravvivano al
`send`/`recv`.

## Perché il progetto reale usa un file temporaneo e non `:memory:`

Questo step usa `Connection::open_in_memory()` per semplicità (niente
file da pulire). Il vero `../../src/db.rs` crea invece un file
temporaneo (`create_temp_db`, con `tempfile::Builder`) sotto
`/tmp/{pool}--XXXXXXXX.db`. La ragione pratica: un backup di un pool
con centinaia di filesystem può girare per minuti; se il processo si
blocca o va in crash a metà, un file `.db` ispezionabile con il comando
`sqlite3` da riga di comando è prezioso per il debug — un database
`:memory:` sparisce col processo, un file resta. È un trade-off
operativo, non una necessità del linguaggio.

## Prossimo passo

[Step 8 — Async/await con `tokio`](../step-8/README.md): le query SQL
di questo step sono sincrone (bloccanti). Il vero tool le mescola con
operazioni SSH asincrone. Vediamo `async fn`, `.await`, e come si
confrontano con `asyncio` di Python.
