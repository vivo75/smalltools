# Step 2 — Ownership, struct e `Option`

> Obiettivo: modellare `PoolConfig` (la configurazione di un pool ZFS da
> backuppare) come dato Rust, e capire cosa significa "possedere" un
> valore invece di condividerlo tramite garbage collector.

## Cosa fare

```bash
cd tutorial/step-2
cargo run
cargo test
```

Prova anche a decommentare la riga finale di `main()` che usa `config`
dopo averlo "consumato": vedrai il compilatore rifiutarsi di compilare
con `error[E0382]: borrow of moved value`. È il modo migliore per farsi
un'idea reale di cosa significhi ownership — leggilo, capisci il
messaggio, poi ricommenta la riga.

## Struct vs classi/dataclass

```python
@dataclass
class PoolConfig:
    name: str
    remote_host: str
    remote_pool: str
    local_pool: str
    compress: str = "lz4"
    compress_flags: str | None = None
    keep_snapshots: list[KeepPolicy] = field(default_factory=list)
```

```rust
struct PoolConfig {
    name: String,
    remote_host: String,
    remote_pool: String,
    local_pool: String,
    compress: String,
    compress_flags: Option<String>,
    keep_snapshots: Vec<KeepPolicy>,
}
```

Sono sorprendentemente simili nella forma. Le differenze reali stanno
sotto la superficie:

- Una `@dataclass` Python vive sull'heap, è raggiunta tramite un
  riferimento contato/tracciato dal garbage collector, e più variabili
  possono puntare allo **stesso** oggetto contemporaneamente.
- Una `struct` Rust non ha un layout di memoria "nascosto": i suoi campi
  sono inline, la sua dimensione è nota a compile-time (a meno di `Vec`,
  `String` ecc. che hanno comunque dimensione fissa: puntatore + len +
  capacity, con i dati veri sull'heap). E soprattutto: ogni valore ha
  **un solo proprietario alla volta**.

## Il concetto centrale: ownership

Questa è la regola che non esiste in Python e che cambia il modo di
scrivere codice:

> Ogni valore ha esattamente un proprietario. Quando il proprietario
> esce di scope, il valore viene distrutto. Puoi trasferire l'ownership
> (*move*) oppure prestarlo temporaneamente (*borrow*, `&`/`&mut`), ma
> non puoi avere due proprietari dello stesso valore.

In pratica, in `src/main.rs` di questo step:

```rust
let summary = summarize_and_consume(config); // `config` è MOSSO qui dentro
println!("{}", config.name);                  // ERRORE: config non esiste più
```

`summarize_and_consume(config: PoolConfig)` prende `config` per valore,
quindi ne diventa proprietaria. Quando la funzione finisce, quel valore
viene distrutto (drop) — deterministicamente, nello stesso istante in
cui usciamo dallo scope, non "prima o poi quando gira il GC" come in
Python.

Se invece la funzione avesse preso `&PoolConfig` (un prestito), come fa
`validate`, il chiamante manterrebbe l'ownership e potrebbe continuare a
usare `config` dopo la chiamata.

### Perché a un pythonista importa

In Python non ti sei mai dovuto preoccupare di "chi possiede" un
dizionario: chiunque ci ha un riferimento può leggerlo o modificarlo, e
se due parti del programma si aspettano cose diverse da quell'oggetto
condiviso, lo scopri a runtime (o mai, se sei fortunato). Rust sposta
quella scoperta a compile-time: se una funzione può modificare un dato,
la sua firma lo dichiara (`&mut`); se una funzione ne prende possesso
esclusivo, la firma lo dichiara (nessun `&`). Leggendo solo la firma sai
già cosa può succedere ai tuoi dati.

## Borrowing: `&T` e `&mut T`

| | Python (convenzione) | Rust (garanzia del compilatore) |
|---|---|---|
| Lettura condivisa | qualunque funzione che non riassegna attributi | `&T` — più prestiti immutabili simultanei ammessi |
| Scrittura | qualunque funzione, nessuna distinzione | `&mut T` — **un solo** prestito mutabile alla volta, e nessun prestito immutabile concorrente |

Questa regola ("o molti lettori, o un solo scrittore, mai entrambi
insieme") è quello che elimina intere classi di bug di concorrenza e di
"mutazione a sorpresa" già a compile-time. La rivedremo con più peso
nello [step 8](../step-8/README.md), dove passiamo dati tra task
asincroni.

## `Option<T>` invece di `None` sparso ovunque

Python usa `None` come valore jolly che *qualsiasi* variabile può
assumere, tipizzato o no: `str | None` è solo un annotamento opzionale.
Rust separa nettamente `T` (un valore che c'è sicuramente) da
`Option<T>` (un valore che può esserci o no) come **tipi diversi**: non
puoi passare un `Option<String>` dove serve una `String` senza prima
gestire esplicitamente il caso `None` (qui chiamato `None` anche in
Rust, l'altra variante si chiama `Some(valore)`).

```rust
fn effective_compress_flags(config: &PoolConfig) -> &str {
    match &config.compress_flags {
        Some(flags) => flags.as_str(),
        None => "",
    }
}
```

Equivalente Python:

```python
def effective_compress_flags(config: PoolConfig) -> str:
    return config.compress_flags or ""
```

La differenza pratica: `match` su un `Option` è **esaustivo**. Se domani
`Option` avesse tre varianti invece di due (non è così, ma vale in
generale per gli `enum`, che vedremo negli step successivi), il
compilatore ti obbligherebbe a gestire anche la terza. In Python,
`config.compress_flags or ""` nasconde silenziosamente il caso in cui
`compress_flags` sia una stringa vuota valida ma "falsy" — un piccolo
bug di troncamento della logica che qui il type system rende visibile
perché sei costretto a scrivere il `match` esplicito.

## Perché queste scelte nel progetto reale

`../../src/config.rs` usa esattamente questa struttura (con l'aggiunta
di `serde`, che vedremo nello [step 4](../step-4/README.md)). Il campo
`compress_flags` lì è una `String` con un default vuoto invece di un
`Option<String>` — nel mondo reale, per un flag da riga di comando,
"nessun flag" e "flag vuoto" sono la stessa cosa, quindi non serve la
distinzione a tre stati che `Option` offrirebbe. L'abbiamo modellato con
`Option` qui apposta per introdurre il concetto: nello step 4 vedrai la
scelta pragmatica fatta nel codice reale, e capirai *perché* è
ragionevole semplificare.

## Prossimo passo

[Step 3 — Error handling con `Result`, `?`, `thiserror` e
`anyhow`](../step-3/README.md): `validate` qui restituisce
`Result<(), String>`, un errore "stringa e basta". Vedremo perché nel
tool reale si usa un `enum` di errori tipizzato invece di stringhe
libere, e come il compilatore vi obbliga a gestire ogni possibile
fallimento.
