use crate::gc::ObjString;
use std::alloc::{self, dealloc, Layout};
use std::fmt::Debug;
use std::ptr::{self, null_mut};

enum Entry<T> {
    Empty,
    Tombstone { key: *const ObjString },
    Data { key: *const ObjString, value: T },
}

impl<T: Debug> Debug for Entry<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Entry::Empty => write!(f, "Empty"),
            Entry::Tombstone { key } => {
                let key_value = unsafe { (**key).get_value() };
                let key_hash = unsafe { (**key).get_hash() };
                write!(
                    f,
                    "Tombstone {{ key: {:?}, hash: {:?}, at: {:?} }}",
                    key_value, key_hash, key
                )
            }
            Entry::Data { key, value } => {
                let key_value = unsafe { (**key).get_value() };
                let key_hash = unsafe { (**key).get_hash() };
                write!(
                    f,
                    "Data {{ key: {:?}, hash: {:?} at: {:?}, value: {:?} }}",
                    key_value, key_hash, key, value
                )
            }
        }
    }
}

#[derive(Debug)]
pub struct Table<T: Copy> {
    ptr: *mut Entry<T>,
    cap: usize,
    len: usize,
}

const TABLE_MAX_LOAD: f64 = 0.75;

/// Finds an entry by the key, returns it and the kind of entry found:
///
/// * KeyMatch — existing entry with the same key
/// * EmptyBucket — empty bucket
/// * Tombstone — tombstone
///
/// Panics if cap is 0. Will not halt if the table is full.
unsafe fn get_entry<T>(ptr: *mut Entry<T>, cap: usize, key: *const ObjString) -> *mut Entry<T> {
    let mut index = (*key).get_hash() as usize % cap;
    let mut tombstone: *mut Entry<T> = null_mut();
    loop {
        let entry = ptr.offset(index as isize);
        match *entry {
            // Empty bucket
            Entry::Empty => {
                return if tombstone.is_null() {
                    entry
                } else {
                    tombstone
                }
            }
            Entry::Tombstone { key: k } => {
                // We found a tombstone, so we can reuse it.
                if tombstone == null_mut() {
                    tombstone = entry;
                }
            }
            Entry::Data { key: k, .. } => {
                // We found the key
                if k == key {
                    return entry;
                }
            }
        }
        index = (index + 1) % cap;
    }
}

unsafe fn free_entries<T>(ptr: *mut Entry<T>, cap: usize) {
    dealloc(ptr as *mut u8, Layout::array::<Entry<T>>(cap).unwrap())
}

fn grow_capacity(cap: usize) -> usize {
    if cap < 8 {
        8
    } else {
        cap * 2
    }
}

impl<T: Copy + Debug> Table<T> {
    fn print(&self) {
        println!("Table: {{");
        for i in 0..self.cap {
            unsafe {
                let entry = &*self.ptr.offset(i as isize);
                println!("  {:?}", entry);
            }
        }
        println!("}}");
    }
}

impl<T: Copy> Table<T> {
    pub fn new() -> Self {
        Self {
            ptr: null_mut(),
            cap: 0,
            len: 0,
        }
    }

    unsafe fn adjust_capacity(&mut self, new_cap: usize) {
        assert!(
            new_cap > self.cap,
            "new_cap {} must be greater than current capacity {}",
            new_cap,
            self.cap
        );

        // Allocate new entries
        let new_layout = Layout::array::<Entry<T>>(new_cap).unwrap();
        assert!(
            new_layout.size() <= isize::MAX as usize,
            "allocation too large"
        );
        let new_ptr = alloc::alloc(new_layout) as *mut Entry<T>;
        if new_ptr.is_null() {
            panic!("allocation failed");
        }
        // Set new entries to null
        for i in 0..new_cap {
            new_ptr.offset(i as isize).write(Entry::Empty);
        }

        self.len = 0;
        // Copy old entries to new entries
        for i in 0..self.cap {
            let entry = self.ptr.offset(i as isize);
            if let Entry::Data { key, value } = &*entry {
                let mut dest = get_entry(new_ptr, new_cap, *key);
                (*dest) = Entry::Data {
                    key: *key,
                    value: *value,
                };
                self.len += 1;
            }
        }

        // Free old entries
        free_entries(self.ptr, self.cap);

        // Update table
        self.ptr = new_ptr;
        self.cap = new_cap;
    }

    /// Sets the value of the key in the table. Returns true if the key was
    /// *not* already present in the table.
    ///
    /// Please note that keys are compared using **pointer equality**.
    pub fn set(&mut self, key: &ObjString, value: T) -> bool {
        if self.len + 1 > (self.cap as f64 * TABLE_MAX_LOAD) as usize {
            unsafe {
                self.adjust_capacity(grow_capacity(self.cap));
            }
        }
        unsafe {
            let mut entry = get_entry(self.ptr, self.cap, key);
            let mut result: bool = match *entry {
                Entry::Empty => {
                    // New entry
                    self.len += 1;
                    true
                }
                Entry::Tombstone { key: _ } => true,
                Entry::Data { key: _, value: _ } => false,
            };
            (*entry) = Entry::Data { key, value };
            return result;
        }
    }

    /// Returns the value of the key in the table.
    ///
    /// Please note that keys are compared using **pointer equality**.
    pub fn get(&self, key: &ObjString) -> Option<&T> {
        if self.len == 0 {
            return None;
        }
        unsafe {
            let entry = get_entry(self.ptr, self.cap, key);
            return match &*entry {
                Entry::Data { key: _, value } => Some(&value),
                _ => None,
            };
        }
    }
    /// Returns the value of the key in the table.
    ///
    /// Please note that the keys are compared using **string equality**.
    pub fn find(&self, find_key: &ObjString) -> Option<&T> {
        if self.len == 0 {
            return None;
        }
        let mut index = (*find_key).get_hash() as usize % self.cap;
        loop {
            unsafe {
                let entry = self.ptr.offset(index as isize);
                match &*entry {
                    Entry::Data { key, value } => {
                        if **key == *find_key {
                            return Some(&value);
                        }
                    }
                    Entry::Empty => {
                        return None;
                    }
                    Entry::Tombstone { .. } => {
                        // Skip
                    }
                }
            }
            index = (index + 1) % self.cap;
        }
    }

    /// Delete the key from the table. Returns true if the key was present in
    /// the table.
    ///
    /// Please note that keys are compared using **pointer equality**.
    pub fn delete(&mut self, key: &ObjString) -> bool {
        if self.len == 0 {
            return false;
        }
        unsafe {
            // Find the entry
            let entry = get_entry(self.ptr, self.cap, key);
            return if let Entry::Data { .. } = &*entry {
                // Delete the entry
                *entry = Entry::Tombstone { key };
                true
            } else {
                false
            };
        }
    }
}

impl<T: Copy> Drop for Table<T> {
    fn drop(&mut self) {
        unsafe { free_entries(self.ptr, self.cap) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_set() {
        let mut table = Table::new();

        let foo = ObjString::new("foo".to_string());
        assert!(table.set(&foo, ()));
        assert!(!table.set(&foo, ()));

        let bar = ObjString::new("bar".to_string());
        assert!(table.set(&bar, ()));
        assert!(!table.set(&bar, ()));

        let baz = ObjString::new("baz".to_string());
        assert!(table.set(&baz, ()));
        assert!(!table.set(&baz, ()));
    }

    #[test]
    fn test_setting_256_values() {
        let keys = (0..255)
            .map(|i| (i, ObjString::new(format!("key_{}", i))))
            .collect::<Vec<_>>();
        // We need to use a large number of keys to trigger the capacity
        // adjustment at least a few times.
        let mut table = Table::new();
        for (i, key) in &keys {
            println!("Setting key {:?} at address {:p}", key, key);
            assert!(table.set(key, i));
            assert!(!table.set(key, i));
        }
    }

    #[test]
    fn test_set_get() {
        let mut table = Table::new();

        let foo = ObjString::new("foo".to_string());

        assert!(table.set(&foo, ()));
        assert_eq!(table.get(&foo), Some(&()));
    }

    #[test]
    fn test_empty_get() {
        let table: Table<()> = Table::new();

        let foo = ObjString::new("foo".to_string());

        assert_eq!(table.get(&foo), None);
    }

    #[test]
    fn test_wrong_get() {
        let mut table = Table::new();

        let foo = ObjString::new("foo".to_string());

        assert!(table.set(&foo, ()));

        let bar = ObjString::new("bar".to_string());

        assert_eq!(table.get(&bar), None);
    }

    #[test]
    fn test_getting_256_values() {
        let keys = (0..255)
            .map(|i| (i, ObjString::new(format!("key_{}", i))))
            .collect::<Vec<_>>();
        // We need to use a large number of keys to trigger the capacity
        // adjustment at least a few times.
        let mut table = Table::new();
        for (i, key) in &keys {
            println!("Setting key {:?} at address {:p}", key, key);
            assert!(table.set(&key, *i));
            assert!(!table.set(&key, *i));

            println!("Getting key {:?} at address {:p}", key, key);
            assert_eq!(table.get(&key), Some(i));
        }
    }

    #[test]
    fn test_set_delete() {
        let mut table = Table::new();

        let foo = ObjString::new("foo".to_string());

        assert!(table.set(&foo, ()));
        assert!(!table.set(&foo, ()));
        assert!(table.delete(&foo));
        assert!(!table.delete(&foo));
        assert_eq!(table.get(&foo), None);
    }

    #[test]
    fn test_empty_delete() {
        let mut table: Table<()> = Table::new();

        let foo = ObjString::new("foo".to_string());

        assert!(!table.delete(&foo));
    }

    #[test]
    fn test_delete_wrong() {
        let mut table = Table::new();

        let foo = ObjString::new("foo".to_string());

        assert!(table.set(&foo, ()));
        let bar = ObjString::new("bar".to_string());

        assert!(!table.delete(&bar));
        assert_eq!(table.get(&foo), Some(&()));
    }

    #[test]
    fn test_delete_256_values() {
        // We need to use a large number of keys to trigger the capacity
        // adjustment at least a few times.
        let mut table = Table::new();
        let keys = (0..256)
            .map(|i| ObjString::new(format!("key_{}", i)))
            .collect::<Vec<_>>();

        for key in &keys {
            println!("Setting key {:?} at address {:p}", key, key);
            assert!(table.set(key, ()));
            assert!(!table.set(key, ()));
        }

        for key in &keys {
            println!("Deleting key {:?} at address {:p}", key, key);
            assert!(table.delete(key));
            assert_eq!(table.get(key), None);
        }
    }

    #[test]
    fn test_delete_half_values() {
        // We need to use a large number of keys to trigger the capacity
        // adjustment at least a few times.
        let mut table = Table::new();
        let deleted_keys = (0..255)
            .map(|i| ObjString::new(format!("key_{}", i)))
            .collect::<Vec<_>>();
        let spared_keys = (256..512)
            .map(|i| ObjString::new(format!("key_{}", i)))
            .collect::<Vec<_>>();
        let all_keys = deleted_keys.iter().chain(&spared_keys).collect::<Vec<_>>();
        for key in &all_keys {
            println!("Setting key {:?} at address {:p}", key, key);
            assert!(table.set(key, ()));
            assert!(!table.set(key, ()));
        }
        for key in &deleted_keys {
            println!("Deleting key {:?} at address {:p}", key, key);
            assert!(table.delete(key));
            assert_eq!(table.get(key), None);
        }
        for key in &spared_keys {
            println!("Getting key {:?} at address {:p}", key, key);
            assert_eq!(table.get(key), Some(&()));
        }
    }

    #[test]
    fn test_delete_8_values_then_add_16() {
        let mut table: Table<isize> = Table::new();
        let first_keys = (0..8)
            .map(|i| (i, ObjString::new(format!("key_{}", i))))
            .collect::<Vec<_>>();

        for (i, key) in &first_keys {
            println!("Setting key {:?} at address {:p}", key, key);
            assert!(table.set(key, *i));
            assert!(!table.set(key, *i));
        }

        println!("Table after adding all the keys:\n{:?}", table);
        table.print();

        for (_, key) in &first_keys {
            println!("Deleting key {:?} at address {:p}", key, key);
            assert!(table.delete(key));
            assert_eq!(table.get(key), None);
        }

        println!("Table after deleting all the keys:\n{:?}", table);
        table.print();

        // Second keys are twice longer and will need to use the tombstones.
        let second_keys = (0..16)
            .map(|i| (i, ObjString::new(format!("key_{}", i))))
            .collect::<Vec<_>>();

        for (i, key) in &second_keys {
            println!("Setting key {:?} at address {:p}", key, key);
            assert!(table.set(key, *i));
            assert!(!table.set(key, *i));
        }

        println!(
            "Table after adding all the keys the second time:\n{:?}",
            table
        );
        table.print();

        for (i, key) in &second_keys {
            println!("Getting key {:?} at address {:p}", key, key);
            assert_eq!(table.get(key), Some(i));
        }
    }

    #[test]
    fn test_set_find() {
        let mut table = Table::new();
        let foo = ObjString::new("foo".to_string());
        assert!(table.set(&foo, ()));
        let foo_2 = ObjString::new("foo".to_string());
        assert_eq!(table.find(&foo_2), Some(&()));
    }

    #[test]
    fn test_empty_find() {
        let mut table: Table<()> = Table::new();
        let foo = ObjString::new("foo".to_string());
        assert_eq!(table.find(&foo), None);
    }

    #[test]
    fn test_wrong_find() {
        let mut table = Table::new();
        let foo = ObjString::new("foo".to_string());
        assert!(table.set(&foo, ()));
        let bar = ObjString::new("bar".to_string());
        assert_eq!(table.find(&bar), None);
    }

    #[test]
    fn test_finding_256_values() {
        let mut table: Table<usize> = Table::new();
        let keys = (0..256)
            .map(|i| (i, ObjString::new(format!("key_{}", i))))
            .collect::<Vec<_>>();
        for (i, key) in &keys {
            println!("Setting key {:?} at address {:p} to value {}", key, key, i);
            assert!(table.set(key, *i));
        }

        println!("Table after adding all the keys:\n{:?}", table);
        table.print();

        for (i, _) in &keys {
            let key = ObjString::new(format!("key_{}", i));
            println!(
                "Getting key {:?} at address {:p}, expecting value {}",
                key, &key, i
            );
            assert_eq!(table.find(&key), Some(i));
        }
    }
}
