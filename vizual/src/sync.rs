use std::time::Duration;

use color_eyre::eyre::{Result, eyre};
use tokio::sync::Mutex as TokioMutex;

pub type MutexGuard<'a, T> = tokio::sync::MutexGuard<'a, T>;

pub trait Thread_safe: Send + Sync + 'static {}
impl<T: Send + Sync + 'static> Thread_safe for T {}

pub struct Mutex<T: ?Sized> {
    inner: TokioMutex<T>,
}

impl<T> Mutex<T> {
    pub fn new(value: T) -> Self {
        Self {
            inner: TokioMutex::new(value),
        }
    }
}

impl<T: ?Sized> Mutex<T> {
    pub async fn lock(&self) -> Result<MutexGuard<'_, T>> {
        match tokio::time::timeout(Duration::from_secs(5), self.inner.lock()).await {
            Ok(guard) => Ok(guard),
            Err(_) => {
                self.debugger_breakpoint();
                Err(eyre!("tokio mutex lock timed out after 5 seconds"))
            }
        }
    }

    #[inline(never)]
    fn debugger_breakpoint(&self) {
        // Put your debugger breakpoint here.
        eprintln!("tokio mutex lock timed out after 5 seconds");
    }
}
