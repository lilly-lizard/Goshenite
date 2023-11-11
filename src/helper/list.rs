/// Returns the closest index in `list` to `target_index` unless list is empty.
/// One situation where this is useful is in a gui when you remove an element form a list. It feels
/// nice for the next selected element to be close to the previously selected element.
pub fn choose_closest_valid_index(list_length: usize, target_index: usize) -> Option<usize> {
    // empty list -> no valid index
    if list_length == 0 {
        return None;
    }

    // no larger indices -> return end of list
    if target_index >= list_length {
        let closest_index = list_length - 1;
        return Some(closest_index);
    }

    // otherwise same index in list as before.
    return Some(target_index);
}
