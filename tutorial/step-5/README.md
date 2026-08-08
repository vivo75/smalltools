# Step 5 — Trait, generics e dispatch statico/dinamico

> Obiettivo: capire cosa sono davvero i trait (li avevamo già usati via
> `#[derive(...)]` senza guardarci dentro), come si scrivono a mano, e
> la differenza tra generics (dispatch statico) e trait object
> (dispatch dinamico) — il corrispettivo Rust del duck typing Python.

## Cosa fare

```bash
cd tutorial/step-5
cargo run
cargo test
```

## Da dove nasce l'esempio

`../../src/backup.rs`, funzione `discover_remote_fs`, fa questo per
ogni riga prodotta da `zfs list -Hp`:

```rust
let parts: Vec<&str> = line.split('\t').collect();
if parts.len() >= 6 {
    let fs = db::Filesystem {
        guid: parts[0].to_string(),
        createtxg: parts[1].to_string(),
        // ...
    };
}
```

Funziona, ma indicizzare `parts[0]`, `parts[1]`... è fragile e non
comunica intenzione. In questo step riscriviamo la stessa logica di
parsing usando il trait `FromStr`, che è l'idioma standard di Rust per
"costruisci Self da una stringa" — lo stesso meccanismo dietro
`"42".parse::<i32>()`.

## Cos'è un trait, davvero

```rust
trait Reportable {
    fn report(&self) -> String;
}
```

Un trait dichiara un **insieme di metodi che un tipo si impegna a
implementare**. È l'equivalente più vicino a un'interfaccia Java/Go o a
una classe astratta Python (`ABC`), con una somiglianza ancora più
stretta con `typing.Protocol`:

```python
class Reportable(Protocol):
    def report(self) -> str: ...
```

La differenza pratica principale rispetto a `Protocol` (che è comunque
verificato solo da `mypy`, a parte) è che il compilatore Rust verifica
**sempre**, per ogni chiamata, che il tipo implementi davvero il trait
richiesto — non c'è un percorso di esecuzione che bypassa il controllo.

## `FromStr`: parsing come trait di libreria, non convenzione

```rust
impl FromStr for Filesystem {
    type Err = ParseFsLineError;
    fn from_str(line: &str) -> Result<Self, Self::Err> { ... }
}
```

Da questo momento, `line.parse::<Filesystem>()` funziona esattamente
come `"42".parse::<i32>()`: stesso trait (`FromStr`), stesso metodo
(`parse`), stessa forma di errore (`Result<Self, Self::Err>`). Non
serve inventare un nome di metodo (`Filesystem.from_line`,
`Filesystem.parse_zfs_output`, ognuno diverso da progetto a progetto,
come capita spesso in Python): chi conosce Rust riconosce
immediatamente cosa fa `.parse()` senza leggere la documentazione di
*questo* tipo specifico.

## `Display` vs `Debug`: due trait, due scopi

```rust
impl fmt::Display for Filesystem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({}, {} bytes used)", self.name, self.fs_type, self.used)
    }
}
```

- `Display` → `println!("{fs}")`: output pensato per un umano.
  Equivalente a `__str__`.
- `Debug` (quasi sempre `#[derive(Debug)]`, come negli step precedenti)
  → `println!("{fs:?}")`: output diagnostico, mostra la struttura
  interna. Equivalente a `__repr__`.

Rust separa i due scopi in **due trait distinti**, e il compilatore ti
avvisa se provi a usare `{}` su un tipo che non implementa `Display`
(anche se implementa `Debug`): non puoi confonderli per sbaglio come
capita in Python quando `__str__` non è definito e Python silenziosamente
ricade su `__repr__`.

## Generics: `impl Trait` come parametro (dispatch statico)

```rust
fn print_report(item: &impl Reportable) {
    println!("{}", item.report());
}
```

Questa è zucchero sintattico per una funzione generica:

```rust
fn print_report<T: Reportable>(item: &T) { ... }
```

Il compilatore genera una copia specializzata di questa funzione **per
ogni tipo concreto** con cui viene effettivamente chiamata
(monomorfizzazione) — un po' come se, per ogni tipo che usi con una
funzione Python generica scritta con `TypeVar`, il compilatore
producesse una versione dedicata e ottimizzata invece di risolvere i
metodi dinamicamente a ogni chiamata. Risultato: zero overhead a
runtime, ma il set di tipi concreti dev'essere noto a compile-time.

## Trait object: `dyn Trait` (dispatch dinamico)

```rust
let mixed: Vec<Box<dyn Reportable>> = vec![
    Box::new(filesystems[0].clone()),
    Box::new(snap),
];
```

Qui `Filesystem` e `SnapshotSummary` sono tipi completamente diversi,
ma possono stare nello stesso `Vec` perché sono "impacchettati" dietro
`Box<dyn Reportable>` — un puntatore all'oggetto più un puntatore alla
sua vtable, risolta a runtime. Questo *è* concettualmente il duck
typing di Python (`[filesystem, snapshot]` con entrambi che hanno un
metodo `.report()`), con un vincolo in più: un `dyn Reportable` può
chiamare **solo** i metodi di `Reportable`, non qualsiasi attributo
esista sull'oggetto originale. In Rust il duck typing "libero" non
esiste: o è generico e verificato a compile-time (`impl Trait` /
`T: Trait`), o è dinamico ma comunque delimitato da un trait esplicito
(`dyn Trait`). Non c'è una terza via "qualunque metodo tu abbia, va
bene", che è invece il default in Python.

## `collect()` generico e gestione bulk degli errori

```rust
fn parse_all_strict<T: FromStr>(text: &str) -> Result<Vec<T>, T::Err> {
    text.lines().map(str::parse).collect()
}
```

`collect()` è probabilmente il metodo più "magico" (in senso buono) di
Rust: il suo tipo di ritorno decide **cosa fare** con l'iteratore.
Raccolto in `Vec<T>`, produce un vettore. Raccolto in
`Result<Vec<T>, E>`, si comporta come un "and-then" a catena: si ferma
al primo `Err` e lo restituisce, altrimenti produce `Ok(vec![...])`
con tutti i valori. Lo stesso codice, cambiando solo l'annotazione di
tipo del chiamante, farebbe collect in una `HashMap`, una `HashSet`,
una `String`... In Python l'equivalente più vicino sarebbe scegliere
tra una list comprehension (che si ferma comunque al primo errore non
gestito, ma solleva un'eccezione anziché restituire un valore) o un
ciclo esplicito con `try/except: continue` per il comportamento
"tollerante" di `parse_all_lenient` — qui entrambi i comportamenti sono
espressi come una singola espressione componibile.

## Perché queste scelte nel progetto reale

Il codice reale in `backup.rs` **non** usa `FromStr` — indicizza
`parts[N]` a mano. È una scelta pragmatica valida per un tool piccolo e
stabile (il formato di `zfs list -Hp` è fisso, cambia raramente), ma è
esattamente il tipo di codice che, crescendo, beneficerebbe del
refactoring mostrato qui: se il numero di campi cambiasse (es.
aggiungendo una colonna), il compilatore ti direbbe subito, in un solo
posto (`impl FromStr`), invece di dover ricontrollare ogni punto del
codice che fa `parts[5]`. Vale la pena conoscere questo pattern anche
se non è (ancora) nel codice reale: è quello che scriveresti se questo
parsing dovesse diventare più complesso o riusabile.

## Prossimo passo

[Step 6 — Moduli e testing](../step-6/README.md): fin qui ogni step è
stato un unico file. Ora vediamo come Rust organizza codice su più file
e cartelle (`mod`, `pub`, `lib.rs`), e come `cargo test` distingue unit
test e integration test — mettendo le basi per riorganizzare il codice
come nel progetto reale (`src/lib.rs` + `tests/`).
