// `main.rs`, pur stando nello stesso package Cargo di `lib.rs`, è
// tecnicamente una crate distinta: per usare la libreria deve
// importarla con `use step6_modules_and_testing::...`, esattamente
// come farebbe un consumatore esterno o `tests/integration_test.rs`.
// Vale la pena scriverlo per intero una volta per capirlo bene, invece
// di darlo per scontato.
use step6_modules_and_testing::config;

fn main() {
    let cfg = config::new_pool_config("  TANK  ", "backup.example.com", "tank", "backup/tank");
    println!("normalized name: '{}'", cfg.name);

    match config::validate_pool_config(&cfg) {
        Ok(()) => println!("config is valid"),
        Err(e) => eprintln!("invalid config: {e}"),
    }

    // La riga seguente, se decommentata, NON compila: `util` non è
    // stato dichiarato `pub` in lib.rs, quindi è invisibile da qui.
    //
    //     step6_modules_and_testing::util::normalize_pool_name("x");
    //     // error[E0603]: module `util` is private
}
