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
    error!("failed to {} while: {}", failed_to, error);
    if let Some(source) = error.source() {
        // log raw source error contents
        error!("source error(s):");
        let depth: usize = 0;
        error!("\t{}: {:?}", depth, source);
        log_error_souce(source, depth + 1);
    }
    // panic
    panic!("failed to {} while: {error:?}", failed_to);
}
#[inline]
#[track_caller]
fn log_error_souce(e: &dyn std::error::Error, depth: usize) {
    if let Some(source) = e.source() {
        error!("\t{}: {:?}", depth, source);
        log_error_souce(source, depth + 1);
    }
}
