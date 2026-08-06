use st_zfs_send_recv::config::{KeepPolicy, PoolConfig};

#[test]
fn test_pool_config_creation() {
    let config = PoolConfig {
        name: "test".to_string(),
        remote_host: "backup.example.com".to_string(),
        remote_pool: "tank".to_string(),
        local_pool: "backup/tank".to_string(),
        compress: "lz4".to_string(),
        compress_flags: String::new(),
        uncompress_flags: "-dcfm".to_string(),
        keep_snapshots: vec![
            KeepPolicy {
                count: 7,
                label: "daily".to_string(),
            },
        ],
        skip_filesystems: vec!["docker".to_string()],
    };

    assert_eq!(config.remote_host, "backup.example.com");
    assert_eq!(config.remote_pool, "tank");
    assert_eq!(config.local_pool, "backup/tank");
    assert_eq!(config.compress, "lz4");
    assert_eq!(config.keep_snapshots.len(), 1);
    assert_eq!(config.skip_filesystems.len(), 1);
}

#[test]
fn test_pool_config_defaults() {
    let config = PoolConfig {
        name: "test".to_string(),
        remote_host: "host".to_string(),
        remote_pool: "pool".to_string(),
        local_pool: "backup".to_string(),
        compress: "lz4".to_string(),
        compress_flags: String::new(),
        uncompress_flags: "-dcfm".to_string(),
        keep_snapshots: vec![],
        skip_filesystems: vec![],
    };

    assert_eq!(config.compress, "lz4");
    assert_eq!(config.uncompress_flags, "-dcfm");
}
