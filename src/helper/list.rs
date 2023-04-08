/// Returns the closest index in `list` to `target_index` unless list is empty.
/// One situation where this is useful is in a gui when you remove an element form a list. It feels
/// nice for the next selected element to be close to the previously selected element.
pub fn choose_closest_valid_index<T>(list: &Vec<T>, target_index: usize) -> Option<usize> {
    let list_len = list.len();

    // empty list -> no valid index
    if list_len == 0 {
        return None;
    }

    // no larger indices -> return end of list
    if target_index >= list_len {
        let closest_index = list_len - 1;
        return Some(closest_index);
    }

    // otherwise same index in list as before.
    return Some(target_index);
}
