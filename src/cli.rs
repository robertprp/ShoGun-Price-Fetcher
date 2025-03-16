use clap::{Parser, ValueEnum};

#[rustfmt::skip]
#[derive(Parser)]
#[clap(
    name = "Price Fetcher",
    about = "Shogun price fetcher for Defillama or other price providers",
    author = "Roberto Perez <me@rbus.me>",
    version = "0.1.0"
)]
pub struct Cli {
    #[clap(
        long,
        required = false,
        default_value = "config.toml",
        help = "Path to the configuration file"
    )]
    pub config: String,
    #[clap(
        long, 
        value_enum,
        default_value_t = LogLevel::Info, 
        help = "Log level"
    )]
    pub log_level: LogLevel,
}

/// Log levels which allow to specify the verbosity of the logs output.
#[derive(Debug, Clone, ValueEnum)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl From<LogLevel> for tracing::Level {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Trace => tracing::Level::TRACE,
            LogLevel::Debug => tracing::Level::DEBUG,
            LogLevel::Info => tracing::Level::INFO,
            LogLevel::Warn => tracing::Level::WARN,
            LogLevel::Error => tracing::Level::ERROR,
        }
    }
}
