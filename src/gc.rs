use core::fmt::{Display, Error, Formatter};
use std::marker::PhantomData;
use std::ops::Deref;
use std::ptr::{null_mut, NonNull};

#[derive(PartialEq, Debug)]
pub enum GCValue {
    String { value: String, hash: u32 },
}

struct GCRefInner {
    value: GCValue,
    next: *mut GCRefInner,
}

#[derive(Debug, PartialEq, PartialOrd, Copy, Clone)]
pub struct GCRef {
    ptr: *mut GCRefInner,
    _marker: PhantomData<()>,
}

impl Deref for GCRef {
    type Target = GCValue;

    fn deref(&self) -> &GCValue {
        unsafe {
            match &self.ptr.as_ref() {
                Some(inner) => &inner.value,
                None => panic!("GCRef is null"),
            }
        }
    }
}

impl Display for GCRef {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        match self.deref() {
            GCValue::String { value, hash: _ } => value.fmt(f),
        }
    }
}

pub struct GC {
    refs: *mut GCRefInner,
}

fn hash_string(s: &str) -> u32 {
    // FNV-1a hash
    let mut h = 2166136261u32;
    for c in s.as_bytes() {
        h ^= *c as u32;
        h = h.wrapping_mul(16777619);
    }
    return h;
}

impl GC {
    pub fn new() -> GC {
        GC { refs: null_mut() }
    }

    fn alloc_inner(&mut self, value: GCValue) -> GCRef {
        self.refs = Box::into_raw(Box::new(GCRefInner {
            value,
            next: self.refs,
        }));
        GCRef {
            ptr: self.refs,
            _marker: PhantomData,
        }
    }

    pub fn alloc_string(&mut self, value: String) -> GCRef {
        let hash = hash_string(&value);
        self.alloc_inner(GCValue::String { value, hash })
    }

    unsafe fn free_obj(&mut self, ptr: *mut GCRefInner) {
        match &(*ptr).value {
            GCValue::String { value, hash: _ } => drop(value),
        }
        drop(Box::from_raw(ptr))
    }
}

impl Drop for GC {
    fn drop(&mut self) {
        let mut cur = self.refs;
        while !cur.is_null() {
            let next = unsafe { (*cur).next };
            unsafe {
                self.free_obj(cur);
            }
            cur = next;
        }
    }
}

#[macro_export]
macro_rules! assert_eq_str {
    ($ref: expr, $str: expr) => {
        match &*$ref {
            GCValue::String { value, hash: _ } => assert_eq!(value, &$str.to_string()),
            _ => panic!("Expected string"),
        }
    };
}

#[cfg(test)]
mod test {
    #[macro_use]
    use super::*;
    #[test]
    fn allocates_string_drops() {
        let mut gc = GC::new();
        let r = gc.alloc_string("hello world".to_string());
        assert_eq_str!(r, "hello world");
        drop(gc);
    }

    #[test]
    fn allocates_two_strings_drops() {
        let mut gc = GC::new();
        let s1 = gc.alloc_string("hello world".to_string());
        let s2 = gc.alloc_string("hello world".to_string());
        assert_eq_str!(s1, "hello world");
        assert_eq_str!(s2, "hello world");
        drop(gc);
    }
}
