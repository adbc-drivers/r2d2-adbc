// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

//! r2d2 connection pool manager for ADBC connections.
//!
//! This crate provides a connection pool manager implementation for
//! [ADBC (Arrow Database Connectivity)](https://arrow.apache.org/adbc/)
//! using the [r2d2](https://docs.rs/r2d2) connection pooling library.
//!
//! # Example
//!
//! ```no_run
//! use r2d2_adbc::AdbcConnectionManager;
//! # use adbc_core::error::Error;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a database instance (implementation-specific)
//! // let database = ...; // Your ADBC Database implementation
//!
//! // Create the connection manager
//! // let manager = AdbcConnectionManager::new(database);
//!
//! // Create the connection pool
//! // let pool = r2d2::Pool::new(manager)?;
//!
//! // Get a connection from the pool
//! // let conn = pool.get()?;
//! # Ok(())
//! # }
//! ```

use adbc_core::options::{OptionConnection, OptionValue};
use adbc_core::{Connection, Database};
use std::error::Error as StdError;
use std::fmt;

/// An r2d2 connection manager for ADBC connections.
///
/// This manager wraps an ADBC `Database` instance and uses it to create
/// and validate connections for the r2d2 connection pool.
///
/// Connection options can be provided to configure each connection as it's created.
///
/// # Type Parameters
///
/// * `D` - The ADBC Database implementation type
pub struct AdbcConnectionManager<D>
where
    D: Database,
{
    database: D,
    connection_options: Vec<(String, String)>,
}

impl<D> AdbcConnectionManager<D>
where
    D: Database,
{
    /// Creates a new `AdbcConnectionManager` with the given database.
    ///
    /// Connections will be created without any pre-initialization options.
    ///
    /// # Arguments
    ///
    /// * `database` - An ADBC Database instance that will be used to create connections
    ///
    /// # Example
    ///
    /// ```no_run
    /// use r2d2_adbc::AdbcConnectionManager;
    /// # use adbc_core::Database;
    ///
    /// # fn example<D: Database>(database: D) {
    /// let manager = AdbcConnectionManager::new(database);
    /// # }
    /// ```
    pub fn new(database: D) -> Self {
        Self {
            database,
            connection_options: Vec::new(),
        }
    }

    /// Creates a new `AdbcConnectionManager` with the given database and connection options.
    ///
    /// The provided options will be passed to each connection as it's created.
    ///
    /// # Arguments
    ///
    /// * `database` - An ADBC Database instance that will be used to create connections
    /// * `options` - An iterator of key-value pairs to configure each connection
    ///
    /// # Example
    ///
    /// ```no_run
    /// use r2d2_adbc::AdbcConnectionManager;
    /// # use adbc_core::Database;
    ///
    /// # fn example<D: Database>(database: D) {
    /// let options = vec![
    ///     ("isolation_level".to_string(), "read_committed".to_string()),
    ///     ("timeout".to_string(), "30".to_string()),
    /// ];
    /// let manager = AdbcConnectionManager::with_options(database, options);
    /// # }
    /// ```
    pub fn with_options<I>(database: D, options: I) -> Self
    where
        I: IntoIterator<Item = (String, String)>,
    {
        Self {
            database,
            connection_options: options.into_iter().collect(),
        }
    }

    /// Adds a connection option that will be applied to all new connections.
    ///
    /// # Arguments
    ///
    /// * `key` - The option key
    /// * `value` - The option value
    ///
    /// # Example
    ///
    /// ```no_run
    /// use r2d2_adbc::AdbcConnectionManager;
    /// # use adbc_core::Database;
    ///
    /// # fn example<D: Database>(database: D) {
    /// let mut manager = AdbcConnectionManager::new(database);
    /// manager.add_option("isolation_level", "read_committed");
    /// # }
    /// ```
    pub fn add_option(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.connection_options.push((key.into(), value.into()));
    }

    /// Clears all connection options.
    pub fn clear_options(&mut self) {
        self.connection_options.clear();
    }

    /// Returns a reference to the current connection options.
    pub fn options(&self) -> &[(String, String)] {
        &self.connection_options
    }
}

/// Error wrapper for ADBC errors in the r2d2 context.
///
/// This type wraps the ADBC error type to provide a consistent error
/// interface for the r2d2 connection pool.
#[derive(Debug)]
pub struct AdbcError(pub adbc_core::error::Error);

impl fmt::Display for AdbcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ADBC error: {}", self.0)
    }
}

impl StdError for AdbcError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.0)
    }
}

impl From<adbc_core::error::Error> for AdbcError {
    fn from(err: adbc_core::error::Error) -> Self {
        AdbcError(err)
    }
}

impl<D> r2d2::ManageConnection for AdbcConnectionManager<D>
where
    D: Database + Send + Sync + 'static,
    D::ConnectionType: Send + 'static,
{
    type Connection = D::ConnectionType;
    type Error = AdbcError;

    /// Creates a new connection using the underlying ADBC database.
    ///
    /// If connection options were provided, they will be passed to the connection
    /// during initialization.
    fn connect(&self) -> Result<Self::Connection, Self::Error> {
        if self.connection_options.is_empty() {
            self.database.new_connection().map_err(AdbcError::from)
        } else {
            self.database
                .new_connection_with_opts(
                    self.connection_options
                        .iter()
                        .map(|(k, v)| (OptionConnection::from(k.as_str()), OptionValue::from(v.as_str()))),
                )
                .map_err(AdbcError::from)
        }
    }

    /// Validates that the connection is still functional.
    ///
    /// This performs a lightweight check by attempting to create a new statement.
    /// If statement creation succeeds, the connection is considered valid.
    fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        // Attempt to create a statement as a validation check
        // If this succeeds, the connection is considered valid
        conn.new_statement().map(|_| ()).map_err(AdbcError::from)
    }

    /// Performs a quick check to determine if the connection has been broken.
    ///
    /// This is a fast, synchronous check that returns `false` to indicate
    /// the connection should be tested further with `is_valid`.
    ///
    /// Note: ADBC connections don't provide a lightweight broken state check,
    /// so this always returns `false` to defer to the `is_valid` check.
    fn has_broken(&self, _conn: &mut Self::Connection) -> bool {
        // ADBC connections don't have a lightweight way to check if they're broken
        // without actually trying to use them, so we return false here and rely
        // on is_valid() to do the actual validation
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Actual tests would require a concrete ADBC Database implementation
    // These are structural tests to ensure the types are correct

    #[test]
    fn test_error_display() {
        use adbc_core::error::{Error, Status};

        let adbc_err = Error::with_message_and_status("test error", Status::Internal);
        let wrapped_err = AdbcError(adbc_err);

        let display = format!("{}", wrapped_err);
        assert!(display.contains("ADBC error"));
    }

    #[test]
    fn test_error_source() {
        use adbc_core::error::{Error, Status};

        let adbc_err = Error::with_message_and_status("test error", Status::Internal);
        let wrapped_err = AdbcError(adbc_err);

        assert!(wrapped_err.source().is_some());
    }
}
