use core::fmt::{Display, Error, Formatter};
use std::marker::PhantomData;
use std::ops::Deref;
use std::ptr::{null_mut, NonNull};

#[derive(PartialEq, Debug)]
pub enum GCValue {
    String(String),
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
            GCValue::String(s) => s.fmt(f),
        }
    }
}

pub struct GC {
    refs: *mut GCRefInner,
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
        self.alloc_inner(GCValue::String(value))
    }
}

impl Drop for GC {
    fn drop(&mut self) {
        let mut cur = self.refs;
        while !cur.is_null() {
            let next = unsafe { (*cur).next };
            unsafe {
                match (*cur).value {
                    GCValue::String(ref s) => drop(s),
                }
                drop(Box::from_raw(cur));
            }
            cur = next;
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn allocates_string_drops() {
        let mut gc = GC::new();
        let s = gc.alloc_string("hello world".to_string());
        assert_eq!(*s, GCValue::String("hello world".to_string()));
        drop(gc);
    }

    #[test]
    fn allocates_two_strings_drops() {
        let mut gc = GC::new();
        let s1 = gc.alloc_string("hello world".to_string());
        let s2 = gc.alloc_string("hello world".to_string());
        assert_eq!(*s1, GCValue::String("hello world".to_string()));
        assert_eq!(*s2, GCValue::String("hello world".to_string()));
        drop(gc);
    }
}
