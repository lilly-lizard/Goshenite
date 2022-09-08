/// Expands to the common pattern of:
/// `match $res {
/// 	Ok(val) => val,
/// 	Err($e) => $e_process,
/// }`.
/// Takes args `($res:expr, $e_process:expr)` where `$res` returns a result, $e is the error pattern
/// and `$e_process` is the error code that may process `$e`. (Basically unwrap_or_else but without
/// the closure).
macro_rules! unwrap_or_exec {
    ( $res:expr, $e: pat, $e_process:expr ) => {
        match $res {
            Ok(val) => val,
            Err($e) => $e_process,
        }
    };
}
pub(crate) use unwrap_or_exec;
