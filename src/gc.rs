use crate::table::Table;
use crate::value::Value;
use core::fmt::{Display, Error, Formatter};
use std::marker::PhantomData;
use std::ops::Deref;
use std::ptr::null_mut;

#[derive(PartialEq, Debug)]
pub struct ObjString {
    value: String,
    hash: u32,
}

impl ObjString {
    pub fn new(value: String) -> ObjString {
        let hash = hash_string(&value);
        ObjString { value, hash }
    }
    pub fn get_value(&self) -> &String {
        &self.value
    }
    pub fn get_hash(&self) -> u32 {
        self.hash
    }
}

impl Display for ObjString {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        write!(f, "\"{}\"", self.value)
    }
}

#[derive(PartialEq, Debug)]
pub enum Obj {
    String(ObjString),
}

impl Obj {
    pub fn unwrap_string(&self) -> &ObjString {
        match self {
            Obj::String(s) => s,
            _ => panic!("Expected Obj::String"),
        }
    }
}

struct ObjRefInner {
    value: Obj,
    next: *mut ObjRefInner,
}

#[derive(Debug, PartialEq, PartialOrd, Copy, Clone)]
pub struct ObjRef {
    ptr: *mut ObjRefInner,
    _marker: PhantomData<()>,
}

impl Deref for ObjRef {
    type Target = Obj;

    fn deref(&self) -> &Obj {
        unsafe {
            match &self.ptr.as_ref() {
                Some(inner) => &inner.value,
                None => panic!("ObjRef is null"),
            }
        }
    }
}

impl Display for ObjRef {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        match self.deref() {
            Obj::String(obj_string) => obj_string.fmt(f),
        }
    }
}

pub struct GC {
    strings: Table<*mut ObjRefInner>,
    refs: *mut ObjRefInner,
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
        GC {
            refs: null_mut(),
            strings: Table::new(),
        }
    }

    fn alloc_inner(&mut self, value: Obj) -> *mut ObjRefInner {
        self.refs = Box::into_raw(Box::new(ObjRefInner {
            value,
            next: self.refs,
        }));
        self.refs
    }

    pub fn alloc_string(&mut self, value: String) -> ObjRef {
        let obj_string = ObjString::new(value);
        return if let Some(interned) = self.strings.find(&obj_string) {
            ObjRef {
                ptr: *interned,
                _marker: PhantomData,
            }
        } else {
            let new_inner = self.alloc_inner(Obj::String(obj_string));
            unsafe {
                if let Obj::String(inner_string) = &self.refs.as_ref().unwrap().value {
                    self.strings.set(&inner_string, new_inner);
                } else {
                    panic!("Expected Obj::String");
                }
            }
            ObjRef {
                ptr: self.refs,
                _marker: PhantomData,
            }
        };
    }

    unsafe fn free_obj(&mut self, ptr: *mut ObjRefInner) {
        match &(*ptr).value {
            Obj::String(ObjString { value, hash: _ }) => drop(value),
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
            Obj::String(obj_string) => assert_eq!(obj_string.get_value(), &$str.to_string()),
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
