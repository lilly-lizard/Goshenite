use log::error;

/// Unwraps an [`anyhow::Result`] like normal except it calls [`anyhow_panic`] to log the error chain
#[inline]
#[track_caller]
pub fn anyhow_unwrap<T>(result: anyhow::Result<T>, failed_to: &str) -> T {
    match result {
        Ok(x) => x,
        Err(e) => anyhow_panic(&e, failed_to),
    }
}

/// Logs the error and source(s) then panics
#[inline]
#[track_caller]
pub fn anyhow_panic(error: &anyhow::Error, failed_to: &str) -> ! {
    // log error
    log_anyhow_error_and_sources(error, failed_to);
    // panic
    panic!("failed to {} while: {error:?}", failed_to);
}

pub fn log_anyhow_error_and_sources(error: &anyhow::Error, failed_to: &str) {
    error!("failed to {} while: {}", failed_to, error);
    if let Some(source) = error.source() {
        error!("error message stack:");
        log_error_sources(source, 0);
    }
}

#[inline]
#[track_caller]
pub fn log_error_sources(e: &dyn std::error::Error, depth: usize) {
    error!("\t{}: {}", depth, e);
    if let Some(source) = e.source() {
        log_error_sources(source, depth + 1);
    }
}
