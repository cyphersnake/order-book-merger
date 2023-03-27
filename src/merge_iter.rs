use std::{cmp::Reverse, collections::BinaryHeap};

struct Takeble<T, I: Iterator<Item = T>> {
    iter: I,
    next: T,
}

impl<T, I: Iterator<Item = T>> Takeble<T, I> {
    pub fn try_new(mut iter: I) -> Option<Self> {
        let next = iter.next();
        next.map(|next| Self { iter, next })
    }

    pub fn take(self) -> (T, Option<Self>) {
        let Self { next, iter } = self;

        (next, Self::try_new(iter))
    }
}
impl<T: Ord, I: Iterator<Item = T>> PartialEq for Takeble<T, I> {
    fn eq(&self, other: &Self) -> bool {
        self.next == other.next
    }
}
impl<T: Ord, I: Iterator<Item = T>> Eq for Takeble<T, I> {}

impl<T: Ord, I: Iterator<Item = T>> PartialOrd for Takeble<T, I> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl<T: Ord, I: Iterator<Item = T>> Ord for Takeble<T, I> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.next.cmp(&other.next)
    }
}

/// An iterator that merges sorted iterators into one sorted iterator.
///
/// NOTE: I was discouraged that I couldn't easily find such a simple
/// iterator! Perhaps I'll highlight it in a crate later or look for
/// a better one
pub struct MergeSortedIter<T: Ord, I: Iterator<Item = T>> {
    heap: BinaryHeap<Reverse<Takeble<T, I>>>,
}

impl<T: Ord, I: Iterator<Item = T>> MergeSortedIter<T, I> {
    /// Creates a new `MergeSortedIter` by merging the provided sorted iterators.
    pub fn new(sorted_iterators: impl Iterator<Item = I>) -> Self {
        Self {
            heap: sorted_iterators.flat_map(Takeble::try_new).fold(
                BinaryHeap::new(),
                |mut heap, iter| {
                    heap.push(Reverse(iter));
                    heap
                },
            ),
        }
    }
}
impl<T: Ord, I: Iterator<Item = T>> Iterator for MergeSortedIter<T, I> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(not_empty_peekable) = self.heap.pop() {
            let (el, rest) = not_empty_peekable.0.take();
            if let Some(not_empty_rest) = rest {
                self.heap.push(Reverse(not_empty_rest));
            }
            Some(el)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_sorted_iter() {
        let v1 = [1, 3, 5, 7, 9].into_iter();
        let v2 = [2, 4, 6, 8, 10].into_iter();
        let v3 = [11, 13, 15, 17, 19].into_iter();
        let v4 = [12, 14, 16, 18, 20].into_iter();

        let iter = MergeSortedIter::new([v1, v2, v3, v4].into_iter());

        assert_eq!(
            iter.collect::<Vec<_>>(),
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]
        );
    }
}
