// Confronta con `../../../tests/integration_test.rs`: stessa idea,
// stessa struttura. Ogni file dentro `tests/` viene compilato da Cargo
// come una crate BINARIA A SÉ STANTE, che collega
// `step6_modules_and_testing` come dipendenza esterna — non fa parte
// della libreria. Per questo può usare solo l'API `pub`: è un modo per
// far sì che i test di integrazione verifichino esattamente quello che
// un vero utente della libreria potrebbe fare, non i dettagli interni.
use step6_modules_and_testing::config::{
    new_pool_config, validate_pool_config, KeepPolicy, PoolConfig,
};

#[test]
fn full_config_lifecycle() {
    let mut cfg = new_pool_config("Tank", "backup.example.com", "tank", "backup/tank");
    cfg.keep_snapshots.push(KeepPolicy {
        count: 7,
        label: "daily".to_string(),
    });

    assert_eq!(cfg.name, "tank"); // normalizzato da `new_pool_config`
    assert!(validate_pool_config(&cfg).is_ok());
}

#[test]
fn can_construct_pool_config_directly_because_fields_are_pub() {
    // Possibile solo perché ogni campo di `PoolConfig` è dichiarato
    // `pub` in config.rs. Se anche uno solo fosse privato, questa
    // riga non compilerebbe da qui (ma compilerebbe comunque in un
    // unit test dentro config.rs, perché quello sta nella stessa
    // crate della struct).
    let cfg = PoolConfig {
        name: "test".to_string(),
        remote_host: "host".to_string(),
        remote_pool: "pool".to_string(),
        local_pool: "backup".to_string(),
        keep_snapshots: vec![],
    };

    assert!(validate_pool_config(&cfg).is_ok());
}

// Se decommentata, la riga seguente NON compila da questo file:
// `util` non è `pub mod` in lib.rs, quindi è invisibile a qualunque
// crate esterna alla libreria — tests/ incluso.
//
// use step6_modules_and_testing::util::normalize_pool_name;
