use log::error;

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
