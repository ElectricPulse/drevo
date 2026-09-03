use std::{fmt::Display, future::Future, time::Instant};

pub fn log_info(indentation: usize, message: impl Display) {
    ::log::info!("{:indentation$}{message}", "");
}

pub async fn log_duration<Output, Callback, CallbackFuture>(
    indentation: usize,
    name: impl Into<String>,
    callback: Callback,
) -> Output
where
    Callback: FnOnce() -> CallbackFuture,
    CallbackFuture: Future<Output = Output>,
{
    let name = name.into();
    let started = Instant::now();
    let output = callback().await;
    log_info(
        indentation,
        format_args!("{name} took {:?}", started.elapsed()),
    );
    output
}
