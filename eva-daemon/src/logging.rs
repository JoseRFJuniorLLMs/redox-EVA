//! Structured logging for EVA OS
//!
//! Uses the `tracing` crate for structured, leveled logging with
//! support for JSON output and filtering.

use tracing::Level;
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

/// Log output format
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LogFormat {
    /// Human-readable format (default)
    Pretty,
    /// Compact single-line format
    Compact,
    /// JSON format (for log aggregation)
    Json,
}

/// Logging configuration
#[derive(Debug, Clone)]
pub struct LogConfig {
    /// Minimum log level
    pub level: Level,
    /// Output format
    pub format: LogFormat,
    /// Show timestamps
    pub timestamps: bool,
    /// Show source file and line
    pub file_line: bool,
    /// Show thread IDs
    pub thread_ids: bool,
    /// Show span events (enter/exit)
    pub span_events: bool,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: Level::INFO,
            format: LogFormat::Pretty,
            timestamps: true,
            file_line: false,
            thread_ids: false,
            span_events: false,
        }
    }
}

impl LogConfig {
    /// Create a development configuration (verbose, pretty)
    pub fn development() -> Self {
        Self {
            level: Level::DEBUG,
            format: LogFormat::Pretty,
            timestamps: true,
            file_line: true,
            thread_ids: true,
            span_events: true,
        }
    }

    /// Create a production configuration (info level, compact)
    pub fn production() -> Self {
        Self {
            level: Level::INFO,
            format: LogFormat::Compact,
            timestamps: true,
            file_line: false,
            thread_ids: false,
            span_events: false,
        }
    }

    /// Create a JSON configuration (for log aggregation)
    pub fn json() -> Self {
        Self {
            level: Level::INFO,
            format: LogFormat::Json,
            timestamps: true,
            file_line: true,
            thread_ids: true,
            span_events: true,
        }
    }
}

/// Initialize logging with the given configuration
pub fn init(config: LogConfig) {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            EnvFilter::new(format!("eva_daemon={}", config.level))
        });

    let span_events = if config.span_events {
        FmtSpan::NEW | FmtSpan::CLOSE
    } else {
        FmtSpan::NONE
    };

    match config.format {
        LogFormat::Pretty => {
            let fmt_layer = fmt::layer()
                .with_ansi(true)
                .with_target(true)
                .with_level(true)
                .with_thread_ids(config.thread_ids)
                .with_file(config.file_line)
                .with_line_number(config.file_line)
                .with_span_events(span_events);

            if config.timestamps {
                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(fmt_layer)
                    .init();
            } else {
                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(fmt_layer.without_time())
                    .init();
            }
        }
        LogFormat::Compact => {
            let fmt_layer = fmt::layer()
                .compact()
                .with_ansi(true)
                .with_target(false)
                .with_level(true)
                .with_thread_ids(config.thread_ids)
                .with_file(config.file_line)
                .with_line_number(config.file_line)
                .with_span_events(span_events);

            if config.timestamps {
                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(fmt_layer)
                    .init();
            } else {
                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(fmt_layer.without_time())
                    .init();
            }
        }
        LogFormat::Json => {
            let fmt_layer = fmt::layer()
                .json()
                .with_span_events(span_events);

            tracing_subscriber::registry()
                .with(env_filter)
                .with(fmt_layer)
                .init();
        }
    }
}

/// Initialize logging with default configuration
pub fn init_default() {
    init(LogConfig::default());
}

/// Initialize logging from environment
///
/// Uses EVA_LOG_LEVEL and EVA_LOG_FORMAT environment variables
pub fn init_from_env() {
    let level = std::env::var("EVA_LOG_LEVEL")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(Level::INFO);

    let format = match std::env::var("EVA_LOG_FORMAT").as_deref() {
        Ok("json") => LogFormat::Json,
        Ok("compact") => LogFormat::Compact,
        _ => LogFormat::Pretty,
    };

    init(LogConfig {
        level,
        format,
        ..LogConfig::default()
    });
}

/// Convenience macros re-exported from tracing
pub use tracing::{debug, error, info, trace, warn};

/// Span for tracking operations
pub use tracing::span;

/// Instrument attribute for async functions
pub use tracing::instrument;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = LogConfig::default();
        assert_eq!(config.level, Level::INFO);
        assert_eq!(config.format, LogFormat::Pretty);
    }

    #[test]
    fn test_config_development() {
        let config = LogConfig::development();
        assert_eq!(config.level, Level::DEBUG);
        assert!(config.file_line);
    }

    #[test]
    fn test_config_production() {
        let config = LogConfig::production();
        assert_eq!(config.level, Level::INFO);
        assert_eq!(config.format, LogFormat::Compact);
    }
}
