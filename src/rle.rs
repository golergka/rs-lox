use std::cmp::Eq;
use std::marker::Copy;
use std::fmt::Debug;

#[derive(Debug)]
pub struct Rle<T: Eq + Debug + Copy> {
    data: Vec<RleNode<T>>,
    last_value: Option<T>,
}

#[derive(Debug)]
struct RleNode<T> {
    pub value: T,
    pub count: usize,
}

impl<T: Eq + Debug + Copy> Rle<T> {

    pub fn new() -> Rle<T> {
        Rle {
            data: Vec::new(),
            last_value: None,
        }
    }

    pub fn push(&mut self, value: T) {
        if let Some(last_value) = &self.last_value {
            if last_value == &value {
                self.data.last_mut().unwrap().count += 1;
                return;
            }
        }
        self.data.push(RleNode {
            value: value,
            count: 1,
        });
        self.last_value.replace(value);
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        let mut skipped = 0;
        for node in &self.data {
            if skipped + node.count > index {
                return Some(&node.value);
            } else {
                skipped += node.count;
            }
        }
        return None;
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn first_item() {
        let mut rle: Rle<i32> = Rle::new();
        rle.push(1);
        assert_eq!(rle.get(0), Some(&1));
    }

    #[test]
    fn empty() {
        let rle: Rle<i32> = Rle::new();
        assert_eq!(rle.get(0), None);
    }

    #[test]
    fn same_item() {
        let mut rle: Rle<i32> = Rle::new();
        rle.push(1);
        rle.push(1);
        assert_eq!(rle.get(1), Some(&1));
    }

    #[test]
    fn different_item() {
        let mut rle: Rle<i32> = Rle::new();
        rle.push(1);
        rle.push(2);
        assert_eq!(rle.get(1), Some(&2));
    }
    #[test]
    fn after_same_series() {
        let mut rle: Rle<i32> = Rle::new();
        rle.push(1);
        rle.push(1);
        rle.push(1);
        rle.push(2);
        assert_eq!(rle.get(3), Some(&2));
    }
    #[test]
    fn second_series() {
        let mut rle: Rle<i32> = Rle::new();
        rle.push(1);
        rle.push(1);
        rle.push(2);
        rle.push(2);
        assert_eq!(rle.get(3), Some(&2));
    }
}
