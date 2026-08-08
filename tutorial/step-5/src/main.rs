// step-5: trait, generics, dispatch statico vs dinamico. Il caso
// d'uso è preso da `../../../src/backup.rs`: lì, righe come
// "guid\tcreatetxg\tcreation\ttype\tused\tname" (l'output di
// `zfs list -Hp`) vengono spezzate a mano con `line.split('\t')` e
// indicizzate `parts[0]`, `parts[1]`... Qui vediamo l'alternativa
// idiomatica basata sul trait `FromStr`, che è lo stesso meccanismo
// dietro a `"42".parse::<i32>()`.

use std::fmt;
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Clone)]
#[allow(dead_code)] // guid/createtxg/creation rispecchiano lo schema reale ma non servono a questa demo
struct Filesystem {
    guid: String,
    createtxg: String,
    creation: String,
    fs_type: String,
    used: u64,
    name: String,
}

#[derive(Error, Debug)]
enum ParseFsLineError {
    #[error("expected 6 tab-separated fields, found {0}")]
    WrongFieldCount(usize),
    #[error("invalid 'used' value '{0}': not a number")]
    InvalidUsed(String),
}

/// Implementare `FromStr` per un tipo è quello che rende possibile
/// scrivere `line.parse::<Filesystem>()`, esattamente come per i tipi
/// primitivi: `"42".parse::<i32>()`. È un **trait della libreria
/// standard**, non qualcosa di specifico a questa struct: chiunque
/// legga questo codice sa già cosa aspettarsi da `.parse()`, perché lo
/// stesso pattern è ovunque in Rust.
///
/// Equivalente Python più vicino: un metodo di classe
/// `Filesystem.from_line(line: str) -> "Filesystem"`, ma qui non è una
/// convenzione di progetto — è un'interfaccia standard riconosciuta da
/// tutto l'ecosistema (e da `str::parse`, che è generico su di essa).
impl FromStr for Filesystem {
    type Err = ParseFsLineError;

    fn from_str(line: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() != 6 {
            return Err(ParseFsLineError::WrongFieldCount(parts.len()));
        }

        let used: u64 = parts[4]
            .parse()
            .map_err(|_| ParseFsLineError::InvalidUsed(parts[4].to_string()))?;

        Ok(Filesystem {
            guid: parts[0].to_string(),
            createtxg: parts[1].to_string(),
            creation: parts[2].to_string(),
            fs_type: parts[3].to_string(),
            used,
            name: parts[5].to_string(),
        })
    }
}

/// `impl fmt::Display` è quello che rende possibile `println!("{fs}")`
/// invece di `println!("{fs:?}")` (che userebbe `Debug`, pensato per
/// diagnostica, non per output leggibile). Equivalente Python:
/// `__str__` contro `__repr__`. La distinzione tra le due è la stessa
/// intenzione in entrambi i linguaggi — qui è solo imposta da due
/// trait separati invece che da due dunder method per convenzione.
impl fmt::Display for Filesystem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({}, {} bytes used)",
            self.name, self.fs_type, self.used
        )
    }
}

/// Un trait "nostro", che definisce un comportamento condiviso tra tipi
/// altrimenti non imparentati. Il concetto è lo stesso del duck typing
/// Python ("se ha un metodo `.report()`, va bene"), ma qui è
/// **verificato dal compilatore**, non solo alla prima chiamata a
/// runtime.
///
/// ```python
/// class Reportable(Protocol):
///     def report(self) -> str: ...
/// ```
///
/// (`typing.Protocol` è quanto Python si avvicina a questo concetto —
/// e non a caso è arrivato in Python anni dopo che Rust aveva già i
/// trait fin dalla prima versione.)
trait Reportable {
    fn report(&self) -> String;
}

impl Reportable for Filesystem {
    fn report(&self) -> String {
        format!("filesystem {self}")
    }
}

#[derive(Debug, Clone)]
struct SnapshotSummary {
    name: String,
    creation: String,
}

impl Reportable for SnapshotSummary {
    fn report(&self) -> String {
        format!("snapshot {} created {}", self.name, self.creation)
    }
}

/// Dispatch **statico**: `impl Reportable` nella firma dice "un
/// qualunque tipo concreto che implementa Reportable, deciso a
/// compile-time". Il compilatore genera una versione specializzata di
/// questa funzione per ogni tipo concreto con cui viene chiamata
/// (si chiama *monomorfizzazione*) — zero overhead a runtime, ma il
/// tipo concreto dev'essere noto quando si compila.
fn print_report(item: &impl Reportable) {
    println!("{}", item.report());
}

/// Dispatch **dinamico**: `&dyn Reportable` è un "trait object". A
/// runtime porta con sé un puntatore ai dati e uno alla tabella dei
/// metodi (vtable) — un meccanismo concettualmente identico a come
/// Python risolve `obj.report()` cercando il metodo sulla classe
/// dell'oggetto, MA vincolato: un `dyn Reportable` può *solo* chiamare
/// i metodi di `Reportable`, nient'altro, mentre un oggetto Python può
/// sempre ricevere `getattr` arbitrari.
///
/// Questo è ciò che rende possibile una collezione **eterogenea**, con
/// tipi diversi mescolati insieme, cosa che in Rust altrimenti non è
/// permessa (un `Vec<T>` contiene un solo tipo concreto `T`): in
/// Python `[filesystem, snapshot]` è naturale perché ogni elemento di
/// una lista porta con sé il proprio tipo; in Rust serve dichiararlo
/// esplicitamente con `Box<dyn Reportable>`.
fn print_all_reports(items: &[Box<dyn Reportable>]) {
    for item in items {
        println!("{}", item.report());
    }
}

/// Funzione **generica**: `T: FromStr` è un *trait bound*. Legge:
/// "per qualunque tipo T che sappia essere costruito da una stringa".
/// `collect::<Result<Vec<T>, T::Err>>()` è un idioma molto usato in
/// Rust: trasforma un iteratore di `Result<T, E>` in un
/// `Result<Vec<T>, E>` che **si ferma al primo errore** — l'equivalente
/// di un ciclo Python con `try/except` che fa `raise` (e quindi
/// interrompe) al primo elemento non valido, ma espresso come
/// un'unica espressione componibile invece che come ciclo con effetti
/// collaterali.
fn parse_all_strict<T>(text: &str) -> Result<Vec<T>, T::Err>
where
    T: FromStr,
{
    text.lines().map(str::parse).collect()
}

/// Variante "tollerante": scarta le righe non valide invece di fermare
/// tutto, proprio come fa `../../../src/backup.rs` con
/// `if parts.len() >= 6 { ... }` (le righe malformate vengono
/// silenziosamente ignorate). `filter_map` applica la funzione e tiene
/// solo i risultati che sono `Some(...)`: l'equivalente di una list
/// comprehension Python con un `try/except: continue` dentro.
fn parse_all_lenient<T>(text: &str) -> Vec<T>
where
    T: FromStr,
{
    text.lines().filter_map(|line| line.parse().ok()).collect()
}

fn main() {
    let raw = "guid-1\t100\t2024-01-01\tfilesystem\t1048576\ttank/data\n\
                guid-2\t101\t2024-01-02\tfilesystem\t2097152\ttank/backups";

    let filesystems: Vec<Filesystem> = parse_all_strict(raw).expect("well-formed demo data");
    for fs in &filesystems {
        print_report(fs); // dispatch statico
    }

    let snap = SnapshotSummary {
        name: "tank/data@daily-2024-01-01".to_string(),
        creation: "2024-01-01T00:00:00Z".to_string(),
    };

    // Collezione eterogenea: possibile solo con trait object.
    let mixed: Vec<Box<dyn Reportable>> = vec![Box::new(filesystems[0].clone()), Box::new(snap)];
    print_all_reports(&mixed);

    // Riga malformata (5 campi invece di 6): con la versione "lenient"
    // viene semplicemente scartata, non interrompe l'elaborazione.
    let raw_with_bad_line = "guid-3\t102\t2024-01-03\tfilesystem\t4096\n\
                              guid-4\t103\t2024-01-04\tfilesystem\t8192\ttank/ok";
    let survivors: Vec<Filesystem> = parse_all_lenient(raw_with_bad_line);
    println!("survived parsing: {} filesystem(s)", survivors.len());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_well_formed_line() {
        let fs: Filesystem = "g\t1\t2024-01-01\tfilesystem\t1024\ttank/data"
            .parse()
            .unwrap();
        assert_eq!(fs.name, "tank/data");
        assert_eq!(fs.used, 1024);
    }

    #[test]
    fn rejects_wrong_field_count() {
        let err = "too\tfew\tfields".parse::<Filesystem>().unwrap_err();
        assert!(matches!(err, ParseFsLineError::WrongFieldCount(3)));
    }

    #[test]
    fn rejects_non_numeric_used() {
        let err = "g\t1\t2024-01-01\tfilesystem\tNOTANUMBER\ttank/data"
            .parse::<Filesystem>()
            .unwrap_err();
        assert!(matches!(err, ParseFsLineError::InvalidUsed(_)));
    }

    #[test]
    fn display_matches_expected_format() {
        let fs: Filesystem = "g\t1\t2024-01-01\tfilesystem\t1024\ttank/data"
            .parse()
            .unwrap();
        assert_eq!(format!("{fs}"), "tank/data (filesystem, 1024 bytes used)");
    }

    #[test]
    fn strict_parsing_stops_at_first_bad_line() {
        let text = "g\t1\t2024-01-01\tfilesystem\t1024\ttank/data\nbroken";
        let result: Result<Vec<Filesystem>, _> = parse_all_strict(text);
        assert!(result.is_err());
    }

    #[test]
    fn lenient_parsing_skips_bad_lines() {
        let text = "g\t1\t2024-01-01\tfilesystem\t1024\ttank/data\nbroken";
        let result: Vec<Filesystem> = parse_all_lenient(text);
        assert_eq!(result.len(), 1);
    }
}
