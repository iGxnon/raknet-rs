//! Raknet implementation by rust

#![warn(
    clippy::cognitive_complexity,
    clippy::dbg_macro,
    clippy::debug_assert_with_mut_call,
    clippy::doc_link_with_quotes,
    clippy::doc_markdown,
    clippy::empty_line_after_outer_attr,
    clippy::empty_structs_with_brackets,
    clippy::float_cmp,
    clippy::float_cmp_const,
    clippy::float_equality_without_abs,
    keyword_idents,
    missing_copy_implementations,
    missing_debug_implementations,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    non_ascii_idents,
    noop_method_call,
    clippy::option_if_let_else,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::semicolon_if_nothing_returned,
    clippy::unseparated_literal_suffix,
    clippy::shadow_unrelated,
    clippy::similar_names,
    clippy::suspicious_operation_groupings,
    unused_extern_crates,
    unused_import_braces,
    clippy::unused_self,
    clippy::use_debug,
    clippy::used_underscore_binding,
    clippy::useless_let_if_seq,
    clippy::wildcard_dependencies,
    clippy::wildcard_imports
)]
#![feature(impl_trait_in_assoc_type)]
#![feature(type_alias_impl_trait)]
#![feature(ip_bits)]
#![feature(exclusive_range_pattern)]
#![feature(type_changing_struct_update)]
#![feature(coroutines, proc_macro_hygiene, stmt_expr_attributes, gen_future)]

/// Protocol codec
mod codec;
/// Errors
mod errors;
/// Protocol packet
mod packet;
/// Runtime
pub mod rt;
/// Raknet server
mod server;
/// Service
pub mod service;

#[derive(Debug, Clone)]
struct Peer {
    addr: std::net::SocketAddr,
    mtu: u16,
}
