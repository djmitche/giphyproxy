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

# Exercise Notes

## HTTP

It's been a while since I've thought about direct connect proxies.
Rather than refresh my memory, I operated off what is in the assignment and a quick read of MDN.
I tested this manually with `curl` to see that it was at least implementing enough of the protocol to talk to that utility:

```shell
curl --proxytunnel -x http://127.0.0.1:8080 foo.com:1234/abcd
```

In particular, this is why the HTTP parser accepts, and ignores, headers.

This was a form of informal integration testing, done mainly to inform the unit tests that run automatically.
Given more time, it would make sense to use the clients that will actually be connecting to the application as part of automated integration tests.
