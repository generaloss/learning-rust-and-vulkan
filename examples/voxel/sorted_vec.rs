// sorted_vec.rs

pub struct SortedVec<T, F> {
    items: Vec<T>,
    key_fn: F
}

impl<T, F> SortedVec<T, F> where F: Fn(&T) -> i32 {

    pub fn new(key_fn: F) -> Self {
        Self {
            items: Vec::new(),
            key_fn
        }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn get(&self, key: i32) -> Option<&T> {
        let index = self.items.binary_search_by_key(&key, &self.key_fn).ok()?;
        Some(&self.items[index])
    }

    pub fn get_mut(&mut self, key: i32) -> Option<&mut T> {
        let index = self.items.binary_search_by_key(&key, &self.key_fn).ok()?;
        Some(&mut self.items[index])
    }

    pub fn put(&mut self, item: T) {
        let key = (self.key_fn)(&item);
        match self.items.binary_search_by_key(&key, &self.key_fn) {
            Ok(index) => self.items[index] = item,
            Err(index) => self.items.insert(index, item)
        }
    }

    pub fn remove_by_key(&mut self, key: i32) -> Option<T> {
        if let Ok(index) = self.items.binary_search_by_key(&key, &self.key_fn) {
            Some(self.items.remove(index))
        } else {
            None
        }
    }

    pub fn range(&self, mut start_key: i32, mut end_key: i32) -> &[T] {
        if start_key > end_key {
            std::mem::swap(&mut start_key, &mut end_key);
        }

        let start_index = self.items.partition_point(|x| (self.key_fn)(x) < start_key);
        let end_index = self.items.partition_point(|x| (self.key_fn)(x) <= end_key);

        &self.items[start_index..end_index]
    }

    pub fn range_from(&self, start_key: i32) -> &[T] {
        let start_index = self.items.partition_point(|x| (self.key_fn)(x) < start_key);

        &self.items[start_index..]
    }

    pub fn range_to(&self, end_key: i32) -> &[T] {
        let end_index = self.items.partition_point(|x| (self.key_fn)(x) <= end_key);

        &self.items[..end_index]
    }

    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.items.iter()
    }
    
    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, T> {
        self.items.iter_mut()
    }

}