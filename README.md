# giphpyproxy

This repository implements a proxy as described [here](https://signal.org/blog/giphy-experiment/).

## Development

You will need to [install Rust](https://www.rust-lang.org/tools/install).

This is a typical Rust binary crate.
Use `cargo test` to run the tests, and `cargo run` to run the application itself.

In most cases, you will want to run with `RUST_LOG=debug` in order to see debug logging.
The running application listens at http://127.0.0.1:8080, acting as a normal HTTP proxy.

## Deployment

You will need to [install Rust](https://www.rust-lang.org/tools/install).

To build the binary for this proxy, use `cargo build --release`
The result will be at `target/release/giphyproxy`.

The binary accepts the following configuration:

 * `RUST_LOG` - logging configuration; see https://crates.io/crates/env_logger

It listens on the loopback interface, on port 8080.
This is not currently configurable.
