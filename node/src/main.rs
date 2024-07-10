//! Kylix Node CLI library.
#![warn(missing_docs)]

mod chain_spec;
#[macro_use]
mod service;
mod benchmarking;
mod cli;
mod command;
mod rpc;
mod rpc_api;
mod rpc_impl;

fn main() -> sc_cli::Result<()> {
	command::run()
}


