use crate::gc::ObjString;
use crate::value::Value;
use std::alloc::{self, dealloc, Layout};
use std::ptr::{self, null_mut};

#[derive(Debug)]
struct Entry {
    key: *const ObjString,
    value: Value,
}

#[derive(Debug)]
pub struct Table {
    ptr: *mut Entry,
    cap: usize,
    len: usize,
}

const TABLE_MAX_LOAD: f64 = 0.75;

fn is_entry_empty(entry: &Entry) -> bool {
    if !entry.key.is_null() {
        return false;
    }
    match entry.value {
        // Tombstone
        Value::Boolean(true) => false,
        _ => true,
    }
}

#[derive(PartialEq, Eq, Hash, Debug)]
enum FindEntryResult {
    KeyMatch,
    EmptyBucket,
    Tombstone,
}

/// Finds an entry by the key, returns it and the kind of entry found:
///
/// * KeyMatch — existing entry with the same key
/// * EmptyBucket — empty bucket
/// * Tombstone — tombstone
///
/// Panics if cap is 0. Will not halt if the table is full.
unsafe fn find_entry(
    ptr: *mut Entry,
    cap: usize,
    key: *const ObjString,
) -> (*mut Entry, FindEntryResult) {
    let mut index = (*key).get_hash() as usize % cap;
    let mut tombstone: *mut Entry = null_mut();
    loop {
        let entry = ptr.offset(index as isize);
        if (*entry).key == key {
            // We found the key.
            return (entry, FindEntryResult::KeyMatch);
        } else if (&*entry).key.is_null() {
            // We found an empty entry or a tombstone.
            match (&*entry).value {
                // We found a tombstone.
                Value::Boolean(true) => {
                    if tombstone.is_null() {
                        tombstone = entry;
                    }
                }
                // Empty entry.
                Value::Nil => {
                    return if tombstone.is_null() {
                        (entry, FindEntryResult::EmptyBucket)
                    } else {
                        (tombstone, FindEntryResult::Tombstone)
                    }
                }
                // We found a non-empty entry.
                _ => panic!("Unexpected value in table"),
            }
        }
        index = (index + 1) % cap;
    }
}

unsafe fn free_entries(ptr: *mut Entry, cap: usize) {
    dealloc(ptr as *mut u8, Layout::array::<Entry>(cap).unwrap())
}

fn grow_capacity(cap: usize) -> usize {
    if cap < 8 {
        8
    } else {
        cap * 2
    }
}

impl Table {
    pub fn new() -> Self {
        Self {
            ptr: null_mut(),
            cap: 0,
            len: 0,
        }
    }

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

    unsafe fn adjust_capacity(&mut self, new_cap: usize) {
        assert!(
            new_cap > self.cap,
            "new_cap {} must be greater than current capacity {}",
            new_cap,
            self.cap
        );

        // Allocate new entries
        let new_layout = Layout::array::<Entry>(new_cap).unwrap();
        assert!(
            new_layout.size() <= isize::MAX as usize,
            "allocation too large"
        );
        let new_ptr = alloc::alloc(new_layout) as *mut Entry;
        if new_ptr.is_null() {
            panic!("allocation failed");
        }
        // Set new entries to null
        for i in 0..new_cap {
            new_ptr.offset(i as isize).write(Entry {
                key: null_mut(),
                value: Value::Nil,
            });
        }

        self.len = 0;
        // Copy old entries to new entries
        for i in 0..self.cap {
            let entry = self.ptr.offset(i as isize);
            if (*entry).key.is_null() {
                continue;
            }
            let (dest, _) = find_entry(new_ptr, new_cap, (*entry).key);
            (*dest).key = (*entry).key;
            (*dest).value = (*entry).value;
            self.len += 1;
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
    pub fn set(&mut self, key: &ObjString, value: Value) -> bool {
        if self.len + 1 > (self.cap as f64 * TABLE_MAX_LOAD) as usize {
            unsafe {
                self.adjust_capacity(grow_capacity(self.cap));
            }
        }
        unsafe {
            let (mut entry, find_result) = find_entry(self.ptr, self.cap, key);
            (*entry).key = key;
            (*entry).value = value;
            return match find_result {
                FindEntryResult::EmptyBucket => {
                    self.len += 1;
                    true
                }
                FindEntryResult::KeyMatch => false,
                FindEntryResult::Tombstone => true
            };
        }
    }

    /// Returns the value of the key in the table.
    ///
    /// Please note that keys are compared using **pointer equality**.
    pub fn get(&self, key: &ObjString) -> Option<&Value> {
        if self.len == 0 {
            return None;
        }
        unsafe {
            let (entry, find_result) = find_entry(self.ptr, self.cap, key);
            return match find_result {
                FindEntryResult::KeyMatch => Some(&(*entry).value),
                _ => None,
            };
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
            let (entry, find_result) = find_entry(self.ptr, self.cap, key);
            if find_result != FindEntryResult::KeyMatch {
                return false;
            }
            // Place a tombstone in the entry
            (*entry).key = null_mut();
            (*entry).value = Value::Boolean(true);
            return true;
        }
    }
}

impl Drop for Table {
    fn drop(&mut self) {
        unsafe { free_entries(self.ptr, self.cap) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GC;

    #[test]
    fn test_simple_set() {
        let mut table = Table::new();
        let mut gc = GC::new();

        let foo_ref = gc.alloc_string("foo".to_string());
        let foo = &(&*foo_ref).unwrap_string();
        assert!(table.set(foo, Value::Nil));
        assert!(!table.set(foo, Value::Nil));

        let bar_ref = gc.alloc_string("bar".to_string());
        let bar = &(&*bar_ref).unwrap_string();
        assert!(table.set(bar, Value::Nil));
        assert!(!table.set(bar, Value::Nil));

        let baz_ref = gc.alloc_string("baz".to_string());
        let baz = &(&*baz_ref).unwrap_string();
        assert!(table.set(baz, Value::Nil));
        assert!(!table.set(baz, Value::Nil));
    }

    #[test]
    fn test_setting_256_values() {
        let mut gc = GC::new();
        // We need to use a large number of keys to trigger the capacity
        // adjustment at least a few times.
        let mut table = Table::new();
        for i in 0..255 {
            let key_ref = gc.alloc_string(format!("key_{}", i));
            let key = &(&*key_ref).unwrap_string();
            println!("Setting key {:?} at address {:p}", key, &key);
            assert!(table.set(key, Value::Number(i as f32)));
            assert!(!table.set(key, Value::Nil));
        }
    }

    #[test]
    fn test_set_get() {
        let mut table = Table::new();
        let mut gc = GC::new();

        let foo_ref = gc.alloc_string("foo".to_string());
        let foo = &(&*foo_ref).unwrap_string();

        assert!(table.set(foo, Value::Nil));
        assert_eq!(table.get(foo), Some(&Value::Nil));
    }

    #[test]
    fn test_empty_get() {
        let table = Table::new();
        let mut gc = GC::new();

        let foo_ref = gc.alloc_string("foo".to_string());
        let foo = &(&*foo_ref).unwrap_string();

        assert_eq!(table.get(foo), None);
    }

    #[test]
    fn test_wrong_get() {
        let mut table = Table::new();
        let mut gc = GC::new();

        let foo_ref = gc.alloc_string("foo".to_string());
        let foo = &(&*foo_ref).unwrap_string();

        assert!(table.set(foo, Value::Nil));

        let bar_ref = gc.alloc_string("bar".to_string());
        let bar = &(&*bar_ref).unwrap_string();

        assert_eq!(table.get(bar), None);
    }

    #[test]
    fn test_getting_256_values() {
        let mut gc = GC::new();
        // We need to use a large number of keys to trigger the capacity
        // adjustment at least a few times.
        let mut table = Table::new();
        for i in 0..256 {
            let key_ref = gc.alloc_string(format!("key_{}", i));
            let key = &(&*key_ref).unwrap_string();
            println!("Setting key {:?} at address {:p}", key, &key);
            assert!(table.set(key, Value::Nil));
            assert!(!table.set(key, Value::Number(i as f32)));

            println!("Getting key {:?} at address {:p}", key, &key);
            assert_eq!(table.get(key), Some(&Value::Number(i as f32)));
        }
    }

    #[test]
    fn test_set_delete() {
        let mut table = Table::new();
        let mut gc = GC::new();

        let foo_ref = gc.alloc_string("foo".to_string());
        let foo = &(&*foo_ref).unwrap_string();

        assert!(table.set(foo, Value::Nil));
        assert!(!table.set(foo, Value::Nil));
        assert!(table.delete(foo));
        assert!(!table.delete(foo));
        assert_eq!(table.get(foo), None);
    }

    #[test]
    fn test_empty_delete() {
        let mut table = Table::new();
        let mut gc = GC::new();

        let foo_ref = gc.alloc_string("foo".to_string());
        let foo = &(&*foo_ref).unwrap_string();

        assert!(!table.delete(foo));
    }

    #[test]
    fn test_delete_wrong() {
        let mut table = Table::new();
        let mut gc = GC::new();

        let foo_ref = gc.alloc_string("foo".to_string());
        let foo = &(&*foo_ref).unwrap_string();

        assert!(table.set(foo, Value::Nil));
        let bar_ref = gc.alloc_string("bar".to_string());
        let bar = &(&*bar_ref).unwrap_string();

        assert!(!table.delete(bar));
        assert_eq!(table.get(foo), Some(&Value::Nil));
    }

    #[test]
    fn test_delete_256_values() {
        let mut gc = GC::new();
        // We need to use a large number of keys to trigger the capacity
        // adjustment at least a few times.
        let mut table = Table::new();
        let key_refs = (0..256)
            .map(|i| gc.alloc_string(format!("key_{}", i)))
            .collect::<Vec<_>>();

        for key_ref in &key_refs {
            let key = &(&*key_ref).unwrap_string();
            println!("Setting key {:?} at address {:p}", key, &key);
            assert!(table.set(key, Value::Nil));
            assert!(!table.set(key, Value::Nil));
        }

        for i in 0..key_refs.len() {
            let key_ref = &key_refs[i];
            let key = &(&*key_ref).unwrap_string();
            println!("Deleting key {:?} at address {:p}", key, &key);
            assert!(table.delete(key));
            assert_eq!(table.get(key), None);
        }
    }

    #[test]
    fn test_delete_half_values() {
        let mut gc = GC::new();
        // We need to use a large number of keys to trigger the capacity
        // adjustment at least a few times.
        let mut table = Table::new();
        let deleted_key_refs = (0..255)
            .map(|i| gc.alloc_string(format!("key_{}", i)))
            .collect::<Vec<_>>();
        let spared_key_refs = (256..512)
            .map(|i| gc.alloc_string(format!("key_{}", i)))
            .collect::<Vec<_>>();
        let all_key_refs = deleted_key_refs
            .iter()
            .chain(&spared_key_refs)
            .collect::<Vec<_>>();
        for key_ref in &all_key_refs {
            let key = &(&*key_ref).unwrap_string();
            println!("Setting key {:?} at address {:p}", key, &key);
            assert!(table.set(key, Value::Nil));
            assert!(!table.set(key, Value::Nil));
        }
        for i in 0..deleted_key_refs.len() {
            let key_ref = &deleted_key_refs[i];
            let key = &(&*key_ref).unwrap_string();
            println!("Deleting key {:?} at address {:p}", key, &key);
            assert!(table.delete(key));
            assert_eq!(table.get(key), None);
        }
        for spared_key_ref in &spared_key_refs {
            let spared_key = &(&*spared_key_ref).unwrap_string();
            println!("Getting key {:?} at address {:p}", spared_key, &spared_key);
            assert_eq!(table.get(spared_key), Some(&Value::Nil));
        }
    }
    #[test]
    fn test_delete_8_values_then_add_16() {
        let mut gc = GC::new();
        let mut table = Table::new();
        let first_key_refs = (0..8)
            .map(|i| gc.alloc_string(format!("key_{}", i)))
            .collect::<Vec<_>>();

        for i in 0..first_key_refs.len() {
            let key_ref = &first_key_refs[i];
            let key = &(&*key_ref).unwrap_string();
            println!("Setting key {:?} at address {:p}", key, &key);
            assert!(table.set(key, Value::Nil));
            assert!(!table.set(key, Value::Number(i as f32)));
        }

        println!("Table after adding all the keys:\n{:?}", table);
        table.print();

        for i in 0..first_key_refs.len() {
            let key_ref = &first_key_refs[i];
            let key = &(&*key_ref).unwrap_string();
            println!("Deleting key {:?} at address {:p}", key, &key);
            assert!(table.delete(key));
            assert_eq!(table.get(key), None);
        }

        println!("Table after deleting all the keys:\n{:?}", table);
        table.print();

        // Second keys are twice longer and will need to use the tombstones.
        let second_key_refs = (0..16)
            .map(|i| gc.alloc_string(format!("key_{}", i)))
            .collect::<Vec<_>>();

        for i in 0..second_key_refs.len() {
            let key_ref = &second_key_refs[i];
            let key = (&*key_ref).unwrap_string();
            println!("Setting key {:?} at address {:p}", key, &key);
            assert!(table.set(key, Value::Nil));
            assert!(!table.set(key, Value::Number(i as f32)));
        }

        println!(
            "Table after adding all the keys the second time:\n{:?}",
            table
        );
        table.print();

        for i in 0..second_key_refs.len() {
            let key_ref = &second_key_refs[i];
            let key = (&*key_ref).unwrap_string();
            println!("Getting key {:?} at address {:p}", key, key);
            assert_eq!(table.get(key), Some(&Value::Number(i as f32)));
        }
    }
}
