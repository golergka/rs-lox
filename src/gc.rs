use core::fmt::{Display, Formatter, Error};
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
    ptr: NonNull<GCRefInner>,
    _marker: PhantomData<()>,
}

impl Deref for GCRef {
    type Target = GCValue;

    fn deref(&self) -> &GCValue {
        unsafe { &self.ptr.as_ref().value }
    }
}

impl Display for GCRef {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        match self.deref() {
            GCValue::String(s) => s.fmt(f)
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
        let inner = GCRefInner {
            value,
            next: self.refs,
        };
        if self.refs.is_null() {
            self.refs = Box::into_raw(Box::new(inner));
        } else {
            let mut cur = self.refs;
            unsafe {
                while !(*cur).next.is_null() {
                    cur = (*cur).next;
                }
                (*cur).next = Box::into_raw(Box::new(inner));
            }
        }
        GCRef {
            ptr: NonNull::new(self.refs).unwrap(),
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
    fn allocates_string() {
        let mut gc = GC::new();
        let s = gc.alloc_string("hello world".to_string());
        assert_eq!(*s, GCValue::String("hello world".to_string()));
    }

}
