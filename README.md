axum-timeout-rs
-----
A simple web app that uses [axum](https://github.com/tokio-rs/axum) with timeouts.

The project requires the following tools configured on your developer machine:
- Rust compiler and Cargo, check https://www.rust-lang.org/tools/install on how to install both

## How to compile and run
```bash
➜  axum-timeout-rs git:(main) ✗ cargo run
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.17s
     Running `target\debug\axum-timeout-rs.exe`
2024-10-26T06:03:20.292713Z  INFO main ThreadId(01) axum_timeout_rs: src/main.rs:93: Server listening for HTTP on 127.0.0.1:18080
```
