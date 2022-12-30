//!

#![deny(
    clippy::pedantic,
    clippy::match_wildcard_for_single_variants,
    clippy::redundant_closure_for_method_calls
)]
#![warn(
    clippy::perf,
    clippy::complexity,
    clippy::style,
    clippy::suspicious,
    clippy::correctness,
    clippy::module_name_repetitions,
    clippy::similar_names,
    clippy::if_not_else,
    clippy::must_use_candidate,
    clippy::missing_errors_doc,
    clippy::option_if_let_else,
    clippy::match_same_arms,
    clippy::default_trait_access,
    clippy::map_flatten,
    clippy::map_unwrap_or,
    clippy::explicit_iter_loop,
    clippy::too_many_lines,
    clippy::cast_sign_loss,
    clippy::unused_self,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::use_self,
    clippy::needless_borrow,
    clippy::redundant_pub_crate,
    clippy::useless_let_if_seq,
    // missing_docs,
    clippy::upper_case_acronyms
)]
#![forbid(unsafe_code)]
#![allow(clippy::unused_async)]

pub mod client;
#[allow(clippy::module_name_repetitions)]
pub mod objects;
pub mod signer;

use std::sync::Arc;

use anyhow::{Context, Result};
use client::FireblocksClient;

/// Res
///
/// # Errors
/// This function fails if ...
pub fn build() -> Result<Arc<FireblocksClient>> {
    let client = FireblocksClient::new().context("failed to construct client")?;

    Ok(Arc::new(client))
}
