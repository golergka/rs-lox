pub struct Rle<T: std::cmp::Eq> {
    data: Vec<RleNode<T>>,
    last_value: Option<T>,
}

struct RleNode<T> {
    pub value: T,
    pub count: usize,
}

impl<T: std::cmp::Eq> Rle<T> {
    pub fn new() -> Rle<T> {
        Rle {
            data: Vec::new(),
            last_value: None,
        }
    }

    pub fn push(&mut self, value: T) {
        if matches!(&self.last_value, Some(v) if v == &value) {
            self.data.last_mut().unwrap().count += 1;
        } else {
            self.data.push(RleNode {
                value: value,
                count: 1,
            });
        }
    }
    pub fn get(&self, index: usize) -> Option<&T> {
        let mut skipped = 0;
        for node in &self.data {
            if skipped + node.count > index {
                return Some(&node.value)
            } else {
                skipped += node.count;
            }
        };
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
