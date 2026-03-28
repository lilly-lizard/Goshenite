/// Move an item in a sub_slice according to the drag and drop logic.
///
/// Rotates the section of the sub_slice between `source_idx` and `target_idx` such that the item
/// previously at `source_idx` ends up at `target_idx - 1` if `target_idx > source_idx`, and
/// at `target_idx` otherwhise. This matches the expected behavior when grabbing the item in
/// the UI and moving it to another position.
///
/// # Example
///
/// ```rust
/// use egui_dnd::utils::shift_vec;
///
/// let mut v = vec![1, 2, 3, 4];
/// shift_vec(1, 1, &mut v);
/// assert_eq!(v, [1, 2, 3, 4]);
/// shift_vec(0, 2, &mut v);
/// assert_eq!(v, [2, 1, 3, 4]);
/// shift_vec(2, 0, &mut v);
/// assert_eq!(v, [3, 2, 1, 4]);
/// ```
///
/// Returns an error if `source_idx >= len()` or `target_idx > len()`
pub fn shift_slice<T>(
    source_idx: usize,
    target_idx: usize,
    to_shift: &mut [T],
) -> Result<(), ShiftSliceError> {
    if let Some(sub_slice) = to_shift.get_mut(source_idx..target_idx) {
        sub_slice.rotate_left(1.min(sub_slice.len()));
    } else if let Some(sub_slice) = to_shift.get_mut(target_idx..=source_idx) {
        sub_slice.rotate_right(1.min(sub_slice.len()));
    } else {
        return Err(ShiftSliceError::InvalidIndices {
            source_idx,
            target_idx,
            slice_len: to_shift.len(),
        });
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub enum ShiftSliceError {
    InvalidIndices {
        source_idx: usize,
        target_idx: usize,
        slice_len: usize,
    },
}
impl std::fmt::Display for ShiftSliceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidIndices {
                source_idx,
                target_idx,
                slice_len,
            } => {
                write!(
                    f,
                    "Failed to move item from index {} to index {}. Slice has {} elements",
                    source_idx, target_idx, slice_len
                )
            }
        }
    }
}
impl std::error::Error for ShiftSliceError {}
