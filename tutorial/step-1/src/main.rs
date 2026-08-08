// step-1: il minimo indispensabile per orientarsi in un progetto Rust.
//
// Non c'è ancora nulla del vero st-zfs-send-recv: l'obiettivo di questo
// primo passo è solo prendere confidenza con `cargo`, con la sintassi di
// base e con il fatto che il compilatore controlla i tipi *prima* che il
// programma venga eseguito.

use std::path::Path;

/// In Rust ogni funzione dichiara esplicitamente il tipo di ritorno.
/// Non esiste `None` implicito come in Python: se una funzione non
/// restituisce nulla di significativo il tipo è `()` (la "unit type"),
/// l'equivalente più vicino a `None`, ma fa parte del *type system*,
/// non è un valore speciale che spunta a runtime.
fn banner(name: &str) {
    println!("{name} starting");
}

/// `&Path` è un riferimento *in prestito* (borrowed) a un percorso.
/// Non alloca nulla: è più simile a passare una `pathlib.Path` per
/// riferimento in Python, con la differenza che qui il compilatore
/// garantisce che il chiamante non lo modifichi mentre lo leggiamo.
fn config_dir_exists(dir: &Path) -> bool {
    dir.is_dir()
}

fn main() {
    // `let` crea un binding immutabile per default: una volta assegnato,
    // `config_dir` non può essere riassegnato. In Python *tutto* è
    // riassegnabile; qui l'immutabilità è la scelta di default e va
    // esplicitamente disattivata con `let mut` quando serve.
    let config_dir = Path::new("/etc/st-zfs-send-recv.d");

    banner("st-zfs-send-recv");

    // `if` è un'espressione, non solo un'istruzione: può restituire un
    // valore, come l'operatore ternario di Python (`a if cond else b`),
    // ma senza bisogno di sintassi speciale.
    let exit_code = if config_dir_exists(config_dir) {
        println!("Found config directory: {}", config_dir.display());
        0
    } else {
        eprintln!("No config directory at {}", config_dir.display());
        1
    };

    // Niente eccezioni non gestite che terminano il processo con uno
    // stack trace a caso: l'exit code è esplicito, come faresti in
    // Python con `sys.exit(code)`.
    std::process::exit(exit_code);
}

// `#[cfg(test)]` dice al compilatore di includere questo modulo solo
// quando si esegue `cargo test`: non finisce nel binario di release.
// È l'equivalente concettuale di tenere i test in un modulo separato
// che `pytest` scopre da solo, ma qui è il compilatore stesso a
// escluderlo dal build normale, non una convenzione di naming.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_dir_is_false() {
        assert!(!config_dir_exists(Path::new("/does/not/exist/surely")));
    }

    #[test]
    fn existing_dir_is_true() {
        assert!(config_dir_exists(Path::new("/tmp")));
    }
}
