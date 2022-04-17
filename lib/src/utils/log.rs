#![allow(unused_imports)]
use tracing::metadata::LevelFilter;
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use tracing_subscriber::fmt::SubscriberBuilder;
use tracing_subscriber::EnvFilter;
use std::sync::Once;

static SYNC_OBJ: Once = Once::new();

pub fn log_init(verbose: i32, debug: bool) {
    SYNC_OBJ.call_once(move || {
        let mut log_level = match verbose {
            0 => None,
            1 => Some(LevelFilter::WARN),
            2 => Some(LevelFilter::INFO),
            3 => Some(LevelFilter::DEBUG),
            4 => Some(LevelFilter::TRACE),
            _ => None,
        };
        if debug {
            log_level = Some(LevelFilter::DEBUG);
        }

        if let Some(log_level) = log_level {
            SubscriberBuilder::default()
                .with_writer(std::io::stderr)
                .with_max_level(log_level)
                .init();
        } else {
            SubscriberBuilder::default()
                .with_writer(std::io::stderr)
                .with_env_filter(EnvFilter::from_default_env())
                .init();
        }
    });
}

pub fn obscure_error<E>(err: E) -> u16
where
    E: std::error::Error + Sized,
{
    let err = err.to_string();
    let hash = (fxhash::hash32(&err) % (u16::MAX as u32)) as u16;
    debug!("internal error - code={} - {}", hash, err);
    hash
}

pub fn obscure_error_str(err: &str) -> u16 {
    let err = err.to_string();
    let hash = (fxhash::hash32(&err) % (u16::MAX as u32)) as u16;
    debug!("internal error - code={} - {}", hash, err);
    hash
}
