# Step 4 — `serde`: deserializzare YAML senza scriverlo a mano

> Obiettivo: caricare `PoolConfig` da file YAML reali su disco, con
> valori di default, validazione, e capire perché il codice Rust che fa
> tutto questo è più corto (e più sicuro) dell'equivalente Python con
> `yaml.safe_load` + validazione manuale.

## Cosa fare

```bash
cd tutorial/step-4
cargo run    # scansiona una directory temporanea con due file *.pool
cargo test
```

## Il confronto diretto

Python "manuale" (senza pydantic):

```python
import yaml

def load_pool_config(path: Path) -> dict:
    with open(path) as f:
        data = yaml.safe_load(f)

    for required in ("remote_host", "remote_pool", "local_pool"):
        if required not in data or not data[required]:
            raise ValueError(f"{required} is required")

    data.setdefault("compress", "lz4")
    data.setdefault("compress_flags", "")
    data.setdefault("uncompress_flags", "-dcfm")
    data.setdefault("keep_snapshots", [])
    data.setdefault("skip_filesystems", [])
    return data
```

Rust con `serde`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    pub remote_host: String,
    pub remote_pool: String,
    pub local_pool: String,
    #[serde(default = "default_compress")]
    pub compress: String,
    // ...
}

let config: PoolConfig = serde_yaml::from_str(&content)?;
```

La differenza non è solo estetica:

- In Python, `data` resta un `dict` non tipizzato finché non lo
  convalidi a mano: `data["compress"]` compila sempre, anche se hai
  scritto `data["compres"]` per errore — lo scopri solo a runtime, se
  quella riga viene mai eseguita.
- In Rust, `config.compress` non esiste come espressione valida se il
  campo si chiama diversamente: è un errore di compilazione, non un
  `KeyError` (o peggio, un `None` silenzioso da `dict.get`) a runtime.

Con `pydantic` il divario Python si riduce parecchio (validazione
dichiarativa, come qui), ma resta una libreria esterna con overhead di
validazione *a runtime* a ogni istanza. `serde` genera codice
specializzato per `PoolConfig` **a compile-time**: non c'è introspezione
via `__annotations__` mentre il programma gira.

## Come funziona `#[derive(Deserialize)]`

`serde` è una crate insolita: non contiene essa stessa la logica di
parsing YAML/JSON/ecc. Definisce solo i trait `Serialize` e
`Deserialize` (l'interfaccia astratta "so come trasformarmi da/verso un
formato generico"). `serde_yaml`, `serde_json` e simili sono i
*backend* che implementano il formato concreto. `#[derive(Deserialize)]`
è una **macro procedurale**: gira durante la compilazione, legge la
definizione della struct e genera codice Rust equivalente a scrivere a
mano, per ogni campo, "leggi questa chiave dalla mappa, convertila nel
tipo giusto, se manca ed esiste un default usalo, altrimenti errore".

Non è troppo diverso concettualmente da come un decoratore
`@dataclass` genera `__init__`/`__eq__`/`__repr__` ispezionando le
annotazioni di classe — la differenza è *quando* accade: il decoratore
Python genera codice quando il modulo viene importato (a runtime, anche
se una volta sola); la macro Rust genera codice quando il file viene
compilato, e quel codice generato viene esso stesso tipo-controllato e
ottimizzato dal compilatore come se l'avessi scritto a mano.

## `#[serde(default = "fn")]` vs default mutabili condivisi

```rust
#[serde(default)]
pub keep_snapshots: Vec<KeepPolicy>,
```

Nota bene: qui non stiamo scrivendo `keep_snapshots: vec![]` da
qualche parte come valore statico condiviso. Ogni volta che
`serde_yaml::from_str` costruisce un `PoolConfig` senza quella chiave
nello YAML, chiama `Vec::default()` (cioè `Vec::new()`) e ne crea uno
**nuovo**. È l'esatto equivalente del motivo per cui in Python si scrive
`field(default_factory=list)` invece di `field: list = []` in una
dataclass — con la differenza che in Rust questo comportamento corretto
è l'*unico* possibile: la stessa insidia di Python (un `list` di default
mutabile condiviso tra tutte le istanze) non può nemmeno presentarsi,
perché `Vec::new()` alloca sempre un buffer indipendente.

## `#[serde(skip)]`: campi calcolati, non deserializzati

```rust
#[serde(skip)]
pub name: String,
```

Il campo `name` non viene letto dal file YAML (il pool si chiama come
il file, non come un campo al suo interno): lo valorizziamo dopo, in
`load_pool_configs`, da `path.file_stem()`. Serde richiede comunque che
`String` implementi `Default` (lo implementa: stringa vuota) per poter
costruire il valore "vuoto" prima di sovrascriverlo.

## Attraversare una directory: iteratori e `Result` che si propagano

```rust
for entry in std::fs::read_dir(config_dir).context("...")? {
    let entry = entry.context("...")?;
    let path = entry.path();
    ...
}
```

`std::fs::read_dir` restituisce un *iteratore* di `io::Result<DirEntry>`
— ogni singola voce potrebbe fallire indipendentemente (permessi,
race condition sul filesystem...), quindi il `?` interno al loop
gestisce il fallimento di *quella specifica voce*, non dell'intera
scansione. In Python, `os.scandir()` o `Path.iterdir()` sollevano
un'eccezione se la entry sparisce a metà iterazione, e devi avvolgere
il singolo blocco in un `try/except` se vuoi lo stesso livello di
granularità — qui è già incorporato nella forma del tipo restituito
dall'iteratore.

## `TempDir` e RAII: pulizia automatica senza `with`

Nei test e nel `main` di questo step usiamo `tempfile::tempdir()`, che
restituisce un `TempDir`. Non chiamiamo mai esplicitamente una funzione
di cleanup: quando il valore esce di scope, il suo distruttore (il
trait `Drop`, che vedremo meglio più avanti) cancella la directory da
solo. È l'equivalente di:

```python
with tempfile.TemporaryDirectory() as dir_path:
    ...
# cancellata automaticamente all'uscita dal blocco `with`
```

ma senza bisogno di un blocco `with` dedicato: **qualunque** valore
Rust può implementare pulizia automatica alla fine del suo scope,
perché lo scope è già delimitato con precisione dalle regole di
ownership viste nello [step 2](../step-2/README.md). Non è un pattern
riservato ai file: connessioni al database, lock, socket... tutto si
pulisce da solo nello stesso modo.

## Perché queste scelte nel progetto reale

`../../src/config.rs` è, a meno di `tracing::debug!` (step 9), il codice
di questo step. La combinazione `serde` + `#[serde(default = "fn")]` +
validazione manuale post-parsing (`validate_pool_config`) riflette una
scelta precisa: i default *strutturali* (che tipo ha un campo, se è
opzionale) li fa fare a serde; le regole di *business* (remote_host non
può essere vuoto) restano validazione esplicita, perché serde non sa
cosa significhi "vuoto ma non valido" per il tuo dominio — esattamente
come faresti con un `@validator` di pydantic o un controllo manuale
dopo `yaml.safe_load`.

## Prossimo passo

[Step 5 — Trait, generics e Display/From](../step-5/README.md): finora
abbiamo usato trait (`Deserialize`, `Debug`, `Clone`) solo tramite
`#[derive(...)]`, senza sapere davvero cosa sono. Ora li implementiamo a
mano e vediamo come sostituiscono duck typing e classi astratte di
Python.
