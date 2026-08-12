use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub fn get_previous_index(length: usize, index: usize) -> usize {
    if index == 0 {
        return length - 1;
    }

    index - 1
}

pub fn get_next_index(length: usize, mut index: usize) -> usize {
    index += 1;

    if index >= length {
        return 0;
    }

    index
}

pub fn bind_index(length: usize, index: usize) -> usize {
    if index >= length {
        return length - 1;
    }

    index
}

pub fn handle_keys_for_iterable(
    key: &crate::event::Key_event,
    length: usize,
    index: usize,
) -> Option<usize> {
    match key.code {
        crate::event::Key_code::Arrow_left => Some(get_previous_index(length, index)),
        crate::event::Key_code::Arrow_right => Some(get_next_index(length, index)),
        _ => None,
    }
}

pub fn get_string_id(s: impl Into<String>) -> u64 {
    let mut hasher = DefaultHasher::new();
    s.into().hash(&mut hasher);
    hasher.finish()
}
