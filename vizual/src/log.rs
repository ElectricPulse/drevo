use std::{
    fmt::Display,
    future::Future,
    time::{Duration, Instant},
};

pub fn log_info(indentation: usize, message: impl Display) {
    ::log::info!("{:indentation$}{message}", "");
}

pub async fn log_duration<Output, Callback, CallbackFuture>(
    indentation: usize,
    name: impl Into<String>,
    log_started: bool,
    callback: Callback,
) -> Output
where
    Callback: FnOnce() -> CallbackFuture,
    CallbackFuture: Future<Output = Output>,
{
    let name = name.into();
    if log_started {
        log_info(indentation, format_args!("{name} started"));
    }
    let started = Instant::now();
    let output = callback().await;
    log_info(
        indentation,
        format_args!("{name} finished in {:?}", started.elapsed()),
    );
    output
}

pub async fn log_timeout<Output, Callback, CallbackFuture>(
    indentation: usize,
    name: impl Into<String>,
    timeout: Duration,
    callback: Callback,
) -> Output
where
    Callback: FnOnce() -> CallbackFuture,
    CallbackFuture: Future<Output = Output>,
{
    let started = Instant::now();
    let output = callback().await;
    let elapsed = started.elapsed();
    if elapsed > timeout {
        let name = name.into();
        log_info(indentation, format_args!("{name} took {elapsed:?}"));
    }
    output
}
