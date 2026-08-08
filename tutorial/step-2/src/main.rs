// step-2: modelliamo la configurazione di un pool ZFS con delle struct,
// e usiamo questo esempio concreto per capire ownership, borrowing e
// `Option<T>`. È il primo pezzo di dominio reale del tool: confronta
// questo file con `../../src/config.rs` per vedere dove arriveremo
// (lì si aggiunge solo serde, la forma dei dati è già questa).

/// Una struct Rust somiglia a una `@dataclass` Python, ma senza
/// generazione di codice a runtime: i campi, i loro tipi e la memoria
/// occupata sono decisi interamente a compile-time.
///
/// ```python
/// @dataclass
/// class KeepPolicy:
///     count: int
///     label: str
/// ```
#[derive(Debug, Clone)]
struct KeepPolicy {
    count: u32,
    label: String,
}

#[derive(Debug, Clone)]
struct PoolConfig {
    name: String,
    remote_host: String,
    remote_pool: String,
    local_pool: String,
    compress: String,
    // `Option<String>` invece di `str | None` di Python: l'assenza di
    // valore è parte del tipo, non un caso speciale che scopri solo
    // leggendo la documentazione o il codice.
    compress_flags: Option<String>,
    keep_snapshots: Vec<KeepPolicy>,
}

/// Un "costruttore" in Rust è semplicemente una funzione associata
/// (`impl Blocco { fn new(...) }`) o, come qui per semplicità, una
/// funzione libera. Nota che prende `String` (owned), non `&str`
/// (borrowed): la funzione diventa **proprietaria** dei dati passati.
/// Il chiamante che passa una `String` se ne priva (viene "mossa").
fn new_pool_config(
    name: &str,
    remote_host: &str,
    remote_pool: &str,
    local_pool: &str,
) -> PoolConfig {
    PoolConfig {
        // `.to_string()` alloca una nuova String possibile perché la
        // funzione ha ricevuto solo un prestito (`&str`) e non può
        // "rubare" la String del chiamante.
        name: name.to_string(),
        remote_host: remote_host.to_string(),
        remote_pool: remote_pool.to_string(),
        local_pool: local_pool.to_string(),
        compress: "lz4".to_string(),
        compress_flags: None,
        keep_snapshots: Vec::new(),
    }
}

/// Prende un **prestito immutabile** (`&PoolConfig`): può leggere i
/// campi, ma non modificarli, e soprattutto non ne prende possesso.
/// Il chiamante mantiene l'ownership e può continuare a usare `config`
/// dopo questa chiamata — cosa impossibile se avessimo scritto
/// `fn validate(config: PoolConfig)`.
///
/// Equivalente Python: qualunque funzione che riceve un oggetto e lo
/// legge soltanto. La differenza è che in Python questa garanzia
/// ("questa funzione non modifica l'oggetto") è solo una convenzione;
/// qui è imposta dal compilatore.
fn validate(config: &PoolConfig) -> Result<(), String> {
    if config.remote_host.is_empty() {
        return Err("remote_host is required".to_string());
    }
    if config.remote_pool.is_empty() {
        return Err("remote_pool is required".to_string());
    }
    if config.local_pool.is_empty() {
        return Err("local_pool is required".to_string());
    }
    Ok(())
}

/// `Option<T>` sostituisce sia `None` sia il pattern "valore-o-default"
/// tipico di Python (`flags or ""`, `getattr(obj, "flags", "")`).
/// Il `match` seguente è *esaustivo*: il compilatore rifiuta di
/// compilare se dimentichi un ramo (es. se `Option` avesse una terza
/// variante). In Python un `if x is not None` dimenticato è un bug
/// silenzioso; qui è un errore di compilazione.
fn effective_compress_flags(config: &PoolConfig) -> &str {
    match &config.compress_flags {
        Some(flags) => flags.as_str(),
        None => "",
    }
}

/// Questa funzione **consuma** (prende ownership di) `config`. Dopo
/// averla chiamata con un valore, quel valore non è più utilizzabile
/// dal chiamante: è stato "mosso" (moved) dentro la funzione, ed è
/// stato distrutto alla fine di questo scope. Non serve un `del` o
/// un garbage collector: la memoria viene liberata deterministicamente
/// quando `config` esce di scope, qui a fine funzione.
fn summarize_and_consume(config: PoolConfig) -> String {
    format!(
        "{}: {} -> {} (compress={})",
        config.name, config.remote_host, config.local_pool, config.compress
    )
}

fn main() {
    let mut config = new_pool_config("tank", "backup.example.com", "tank", "backup/tank");
    config.keep_snapshots.push(KeepPolicy {
        count: 7,
        label: "daily".to_string(),
    });

    // `validate` prende in prestito `&config`: `config` resta
    // utilizzabile subito dopo.
    match validate(&config) {
        Ok(()) => println!("config for '{}' is valid", config.name),
        Err(e) => eprintln!("invalid config: {e}"),
    }

    println!(
        "effective compress flags: '{}'",
        effective_compress_flags(&config)
    );

    for policy in &config.keep_snapshots {
        println!(
            "retention policy: keep {} snapshots labeled '{}'",
            policy.count, policy.label
        );
    }

    // Se in questo punto avessimo bisogno di "config" ancora dopo la
    // prossima riga, dovremmo clonarlo esplicitamente PRIMA di
    // consumarlo, perché `summarize_and_consume` ne prende ownership:
    //
    //     let backup = config.clone();
    //     let summary = summarize_and_consume(config);
    //     // ora `backup` è ancora usabile, `config` no.
    //
    // In Python questo problema semplicemente non esiste: ogni
    // variabile è un riferimento condiviso e nulla "sposta"
    // l'ownership. Il rovescio della medaglia è che in Python non hai
    // mai la garanzia statica che nessun altro stia modificando
    // l'oggetto sotto i tuoi piedi; in Rust quella garanzia è gratis.
    let summary = summarize_and_consume(config);
    println!("{summary}");

    // A questo punto `config` non esiste più: la riga seguente, se
    // decommentata, non compila:
    // println!("{}", config.name); // error[E0382]: borrow of moved value: `config`
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_config_passes() {
        let config = new_pool_config("tank", "host", "tank", "backup/tank");
        assert!(validate(&config).is_ok());
    }

    #[test]
    fn empty_remote_host_is_rejected() {
        let config = new_pool_config("tank", "", "tank", "backup/tank");
        let err = validate(&config).unwrap_err();
        assert_eq!(err, "remote_host is required");
    }

    #[test]
    fn default_compress_flags_is_empty_string() {
        let config = new_pool_config("tank", "host", "tank", "backup/tank");
        assert_eq!(effective_compress_flags(&config), "");
    }

    #[test]
    fn explicit_compress_flags_are_returned() {
        let mut config = new_pool_config("tank", "host", "tank", "backup/tank");
        config.compress_flags = Some("--fast".to_string());
        assert_eq!(effective_compress_flags(&config), "--fast");
    }

    #[test]
    fn cloning_lets_you_use_data_after_a_move() {
        let config = new_pool_config("tank", "host", "tank", "backup/tank");
        let kept = config.clone();
        let _summary = summarize_and_consume(config); // `config` è mosso qui
        assert_eq!(kept.name, "tank"); // `kept` è indipendente, ancora valido
    }
}
