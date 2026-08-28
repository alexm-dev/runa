//! main.rs
//! Binary entry point for runa.
//!
//! Everything lives in the library crate so that the binary, the test suite and
//! the benchmark harness share one compilation. See `src/lib.rs`.

#![forbid(unsafe_code)]

fn main() -> std::io::Result<()> {
    runa::run()
}
