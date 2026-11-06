//! Structured log formats, immutable events, and search syntax.
mod event;
mod parse;
mod search;
pub use event::{Event, duration_label, scalar};
pub use parse::{ImportError, parse, parse_cancellable};
pub use search::{Term, matches, search_terms};
#[cfg(test)]
mod tests;
