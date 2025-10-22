# r2d2_adbc

[![Crates.io](https://img.shields.io/crates/v/r2d2_adbc.svg)](https://crates.io/crates/r2d2_adbc)
[![Documentation](https://docs.rs/r2d2_adbc/badge.svg)](https://docs.rs/r2d2_adbc)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)

An [r2d2](https://crates.io/crates/r2d2) connection pool manager for [ADBC (Arrow Database Connectivity)](https://arrow.apache.org/adbc/) connections.

## Overview

This crate provides a connection pool manager implementation that bridges ADBC database drivers with the r2d2 connection pooling library. It allows you to efficiently manage and reuse ADBC database connections in multi-threaded applications.

## Features

- **Generic ADBC Support**: Works with any ADBC `Database` implementation
- **Connection Options**: Configure connections with custom options
- **Thread-Safe**: Full support for multi-threaded connection pooling
- **Type-Safe**: Leverages Rust's type system for compile-time safety

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
r2d2_adbc = "0.1"
adbc_core = "0.20"
```

## Usage

### Basic Usage

```rust
use r2d2_adbc::AdbcConnectionManager;

// Create your ADBC database instance
let database = /* your ADBC Database implementation */;

// Create the connection manager
let manager = AdbcConnectionManager::new(database);

// Create the connection pool
let pool = r2d2::Pool::new(manager)?;

// Get a connection from the pool
let conn = pool.get()?;

// Use the connection
let statement = conn.new_statement()?;
```

### With Connection Options

You can configure connections with options that will be applied to each new connection:

```rust
use r2d2_adbc::AdbcConnectionManager;

let database = /* your ADBC Database implementation */;

// Method 1: Create with options upfront
let options = vec![
    ("autocommit".to_string(), "true".to_string()),
    ("isolation_level".to_string(), "read_committed".to_string()),
    ("timeout".to_string(), "30".to_string()),
];
let manager = AdbcConnectionManager::with_options(database, options);

// Method 2: Add options after creation
let mut manager = AdbcConnectionManager::new(database);
manager.add_option("autocommit", "true");
manager.add_option("read_only", "false");

// Create the pool
let pool = r2d2::Pool::builder()
    .max_size(15)
    .build(manager)?;

// All connections from the pool will have the configured options
let conn = pool.get()?;
```

### Configuring the Pool

The r2d2 pool itself can be configured with various parameters:

```rust
use r2d2_adbc::AdbcConnectionManager;
use std::time::Duration;

let database = /* your ADBC Database implementation */;
let manager = AdbcConnectionManager::new(database);

let pool = r2d2::Pool::builder()
    .max_size(20)                          // Maximum number of connections
    .min_idle(Some(5))                      // Minimum idle connections
    .connection_timeout(Duration::from_secs(30))
    .idle_timeout(Some(Duration::from_secs(600)))
    .max_lifetime(Some(Duration::from_secs(1800)))
    .build(manager)?;
```

## How It Works

### Connection Management

The `AdbcConnectionManager` implements the `r2d2::ManageConnection` trait:

- **`connect()`**: Creates new connections using `Database::new_connection()` or `Database::new_connection_with_opts()` if options are configured
- **`is_valid()`**: Validates connections by attempting to create a statement
- **`has_broken()`**: Quick check for broken connections (defers to `is_valid()` for ADBC)

### Connection Options

Connection options are stored as string key-value pairs and automatically converted to the proper ADBC types (`OptionConnection` and `OptionValue`) when creating connections. Standard ADBC options include:

- `autocommit` - Enable/disable autocommit mode
- `isolation_level` - Set transaction isolation level
- `current_catalog` - Set the current catalog
- `current_schema` - Set the current schema
- `read_only` - Restrict connection to read-only mode
- Driver-specific options are also supported

## Compatibility

- **ADBC**: Compatible with `adbc_core` 0.20.x
- **r2d2**: Compatible with r2d2 0.8.x
- **Rust**: Requires Rust 2024 edition or later

## ADBC Drivers

This crate works with any ADBC driver that implements the `adbc_core::Database` trait. Some available ADBC drivers include:

- PostgreSQL
- SQLite
- Flight SQL
- Snowflake
- And more...

Refer to the [ADBC documentation](https://arrow.apache.org/adbc/) for a complete list of available drivers.

## Example: Complete Application

```rust
use r2d2_adbc::AdbcConnectionManager;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Initialize your ADBC database
    let database = /* initialize your ADBC database */;

    // Create the connection manager with options
    let mut manager = AdbcConnectionManager::new(database);
    manager.add_option("autocommit", "true");

    // Build the connection pool
    let pool = r2d2::Pool::builder()
        .max_size(10)
        .build(manager)?;

    // Use connections from the pool
    for i in 0..5 {
        let conn = pool.get()?;
        println!("Got connection {}", i);

        // Use the connection
        let mut stmt = conn.new_statement()?;
        // Execute queries...
    }

    Ok(())
}
```

## Error Handling

The crate provides an `AdbcError` type that wraps `adbc_core::error::Error` and implements the standard `Error` trait. All connection pool operations return results with this error type.

```rust
match pool.get() {
    Ok(conn) => {
        // Use connection
    }
    Err(e) => {
        eprintln!("Failed to get connection: {}", e);
        if let Some(source) = e.source() {
            eprintln!("Caused by: {}", source);
        }
    }
}
```

## Thread Safety

Both `AdbcConnectionManager` and the resulting connection pool are fully thread-safe and can be shared across threads using `Arc`:

```rust
use std::sync::Arc;
use std::thread;

let pool = Arc::new(r2d2::Pool::new(manager)?);

let mut handles = vec![];
for i in 0..10 {
    let pool = Arc::clone(&pool);
    let handle = thread::spawn(move || {
        let conn = pool.get().unwrap();
        // Use connection in this thread
    });
    handles.push(handle);
}

for handle in handles {
    handle.join().unwrap();
}
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.

## Related Projects

- [r2d2](https://github.com/sfackler/r2d2) - Generic connection pool for Rust
- [Apache Arrow ADBC](https://arrow.apache.org/adbc/) - Arrow Database Connectivity specification
- [adbc_core](https://docs.rs/adbc_core) - Core ADBC types and traits for Rust
