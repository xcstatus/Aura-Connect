fn main() -> iced::Result {
    // `RUST_LOG` + tracing; `log::` is bridged inside `tracing_subscriber::try_init` (default `tracing-log` feature).
    // With `term-prof`, a flame layer is stacked on the same subscriber.
    let _logging = rust_ssh::logging::init().expect("logging init failed");
    rust_ssh::app::run()
}
