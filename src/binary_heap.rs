use std::cmp::Ordering;
use std::fmt;
use std::iter::FusedIterator;
use std::mem::{ManuallyDrop, swap};
use std::num::NonZero;
use std::ops::{Deref, DerefMut};
use std::ptr;

use compare::Compare;
use std::collections::TryReserveError;
use std::slice;
use std::vec::{self, Vec};

/// A priority queue implemented with a binary heap.
///
/// This will be a max-heap.
///
/// It is a logic error for an item to be modified in such a way that the
/// item's ordering relative to any other item, as determined by the [`Ord`]
/// trait, changes while it is in the heap. This is normally only possible
/// through interior mutability, global state, I/O, or unsafe code. The
/// behavior resulting from such a logic error is not specified, but will
/// be encapsulated to the `BinaryHeap` that observed the logic error and not
/// result in undefined behavior. This could include panics, incorrect results,
/// aborts, memory leaks, and non-termination.
///
/// As long as no elements change their relative order while being in the heap
/// as described above, the API of `BinaryHeap` guarantees that the heap
/// invariant remains intact i.e. its methods all behave as documented. For
/// example if a method is documented as iterating in sorted order, that's
/// guaranteed to work as long as elements in the heap have not changed order,
/// even in the presence of closures getting unwinded out of, iterators getting
/// leaked, and similar foolishness.
///
/// # Examples
///
/// ```
/// use std::collections::BinaryHeap;
///
/// // Type inference lets us omit an explicit type signature (which
/// // would be `BinaryHeap<i32>` in this example).
/// let mut heap = BinaryHeap::new();
///
/// // We can use peek to look at the next item in the heap. In this case,
/// // there's no items in there yet so we get None.
/// assert_eq!(heap.peek(), None);
///
/// // Let's add some scores...
/// heap.push(1);
/// heap.push(5);
/// heap.push(2);
///
/// // Now peek shows the most important item in the heap.
/// assert_eq!(heap.peek(), Some(&5));
///
/// // We can check the length of a heap.
/// assert_eq!(heap.len(), 3);
///
/// // We can iterate over the items in the heap, although they are returned in
/// // a random order.
/// for x in &heap {
///     println!("{x}");
/// }
///
/// // If we instead pop these scores, they should come back in order.
/// assert_eq!(heap.pop(), Some(5));
/// assert_eq!(heap.pop(), Some(2));
/// assert_eq!(heap.pop(), Some(1));
/// assert_eq!(heap.pop(), None);
///
/// // We can clear the heap of any remaining items.
/// heap.clear();
///
/// // The heap should now be empty.
/// assert!(heap.is_empty())
/// ```
///
/// A `BinaryHeap` with a known list of items can be initialized from an array:
///
/// ```
/// use std::collections::BinaryHeap;
///
/// let heap = BinaryHeap::from([1, 5, 2]);
/// ```
///
/// ## Min-heap
///
/// Either [`core::cmp::Reverse`] or a custom [`Ord`] implementation can be used to
/// make `BinaryHeap` a min-heap. This makes `heap.pop()` return the smallest
/// value instead of the greatest one.
///
/// ```
/// use std::collections::BinaryHeap;
/// use std::cmp::Reverse;
///
/// let mut heap = BinaryHeap::new();
///
/// // Wrap values in `Reverse`
/// heap.push(Reverse(1));
/// heap.push(Reverse(5));
/// heap.push(Reverse(2));
///
/// // If we pop these scores now, they should come back in the reverse order.
/// assert_eq!(heap.pop(), Some(Reverse(1)));
/// assert_eq!(heap.pop(), Some(Reverse(2)));
/// assert_eq!(heap.pop(), Some(Reverse(5)));
/// assert_eq!(heap.pop(), None);
/// ```
///
/// # Time complexity
///
/// | [push]  | [pop]         | [peek]/[peek\_mut] |
/// |---------|---------------|--------------------|
/// | *O*(1)~ | *O*(log(*n*)) | *O*(1)             |
///
/// The value for `push` is an expected cost; the method documentation gives a
/// more detailed analysis.
///
/// [`core::cmp::Reverse`]: core::cmp::Reverse
/// [`Cell`]: core::cell::Cell
/// [`RefCell`]: core::cell::RefCell
/// [push]: BinaryHeap::push
/// [pop]: BinaryHeap::pop
/// [peek]: BinaryHeap::peek
/// [peek\_mut]: BinaryHeap::peek_mut
pub struct BinaryHeap<T, C> {
    data: Vec<T>,
    cmp: C,
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct MaxComparator;

impl<T: Ord> Compare<T> for MaxComparator {
    fn compare(&self, a: &T, b: &T) -> Ordering {
        a.cmp(b)
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct MinComparator;

impl<T: Ord> Compare<T> for MinComparator {
    fn compare(&self, a: &T, b: &T) -> Ordering {
        b.cmp(a)
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct FnComparator<F>(pub F);

impl<T, F> Compare<T> for FnComparator<F>
where
    F: Fn(&T, &T) -> Ordering,
{
    fn compare(&self, a: &T, b: &T) -> Ordering {
        self.0(a, b)
    }
}

/// Structure wrapping a mutable reference to the greatest item on a
/// `BinaryHeap`.
///
/// This `struct` is created by the [`peek_mut`] method on [`BinaryHeap`]. See
/// its documentation for more.
///
/// [`peek_mut`]: BinaryHeap::peek_mut
pub struct PeekMut<'a, T: 'a, C: 'a + Compare<T>> {
    heap: &'a mut BinaryHeap<T, C>,
    // If a set_len + sift_down are required, this is Some. If a &mut T has not
    // yet been exposed to peek_mut()'s caller, it's None.
    original_len: Option<NonZero<usize>>,
}

impl<T: fmt::Debug, C: Compare<T>> fmt::Debug for PeekMut<'_, T, C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("PeekMut").field(&self.heap.data[0]).finish()
    }
}

impl<T, C: Compare<T>> Drop for PeekMut<'_, T, C> {
    fn drop(&mut self) {
        if let Some(original_len) = self.original_len {
            // SAFETY: That's how many elements were in the Vec at the time of
            // the PeekMut::deref_mut call, and therefore also at the time of
            // the BinaryHeap::peek_mut call. Since the PeekMut did not end up
            // getting leaked, we are now undoing the leak amplification that
            // the DerefMut prepared for.
            unsafe { self.heap.data.set_len(original_len.get()) };

            // SAFETY: PeekMut is only instantiated for non-empty heaps.
            unsafe { self.heap.sift_down(0) };
        }
    }
}

impl<T, C: Compare<T>> Deref for PeekMut<'_, T, C> {
    type Target = T;
    fn deref(&self) -> &T {
        debug_assert!(!self.heap.is_empty());
        // SAFE: PeekMut is only instantiated for non-empty heaps
        unsafe { self.heap.data.get_unchecked(0) }
    }
}

impl<T, C: Compare<T>> DerefMut for PeekMut<'_, T, C> {
    fn deref_mut(&mut self) -> &mut T {
        debug_assert!(!self.heap.is_empty());

        let len = self.heap.len();
        if len > 1 {
            // Here we preemptively leak all the rest of the underlying vector
            // after the currently max element. If the caller mutates the &mut T
            // we're about to give them, and then leaks the PeekMut, all these
            // elements will remain leaked. If they don't leak the PeekMut, then
            // either Drop or PeekMut::pop will un-leak the vector elements.
            //
            // This is technique is described throughout several other places in
            // the standard library as "leak amplification".
            unsafe {
                // SAFETY: len > 1 so len != 0.
                self.original_len = Some(NonZero::new_unchecked(len));
                // SAFETY: len > 1 so all this does for now is leak elements,
                // which is safe.
                self.heap.data.set_len(1);
            }
        }

        // SAFE: PeekMut is only instantiated for non-empty heaps
        unsafe { self.heap.data.get_unchecked_mut(0) }
    }
}

impl<'a, T, C: Compare<T>> PeekMut<'a, T, C> {
    /// Removes the peeked value from the heap and returns it.
    pub fn pop(mut this: PeekMut<'a, T, C>) -> T {
        if let Some(original_len) = this.original_len.take() {
            // SAFETY: This is how many elements were in the Vec at the time of
            // the BinaryHeap::peek_mut call.
            unsafe { this.heap.data.set_len(original_len.get()) };

            // Unlike in Drop, here we don't also need to do a sift_down even if
            // the caller could've mutated the element. It is removed from the
            // heap on the next line and pop() is not sensitive to its value.
        }
        this.heap.pop().unwrap()
    }
}

impl<T: Clone, C: Clone> Clone for BinaryHeap<T, C> {
    fn clone(&self) -> Self {
        BinaryHeap {
            data: self.data.clone(),
            cmp: self.cmp.clone(),
        }
    }

    /// Overwrites the contents of `self` with a clone of the contents of `source`.
    ///
    /// This method is preferred over simply assigning `source.clone()` to `self`,
    /// as it avoids reallocation if possible.
    ///
    /// See [`Vec::clone_from()`] for more details.
    fn clone_from(&mut self, source: &Self) {
        self.data.clone_from(&source.data);
        self.cmp.clone_from(&source.cmp);
    }
}

impl<T, C: Compare<T> + Default> Default for BinaryHeap<T, C> {
    /// Creates an empty `BinaryHeap<T>`.
    #[inline]
    fn default() -> BinaryHeap<T, C> {
        BinaryHeap::new()
    }
}

impl<T: fmt::Debug, C: Compare<T>> fmt::Debug for BinaryHeap<T, C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

struct RebuildOnDrop<'a, T, C: Compare<T>> {
    heap: &'a mut BinaryHeap<T, C>,
    rebuild_from: usize,
}

impl<T, C: Compare<T>> Drop for RebuildOnDrop<'_, T, C> {
    fn drop(&mut self) {
        self.heap.rebuild_tail(self.rebuild_from);
    }
}

impl<T, C: Compare<T> + Default> BinaryHeap<T, C> {
    /// Creates an empty `BinaryHeap` as a max-heap.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use std::collections::BinaryHeap;
    /// let mut heap = BinaryHeap::new();
    /// heap.push(4);
    /// ```
    #[must_use]
    pub fn new() -> BinaryHeap<T, C> {
        BinaryHeap {
            data: vec![],
            cmp: C::default(),
        }
    }

    /// Creates an empty `BinaryHeap` with at least the specified capacity.
    ///
    /// The binary heap will be able to hold at least `capacity` elements without
    /// reallocating. This method is allowed to allocate for more elements than
    /// `capacity`. If `capacity` is 0, the binary heap will not allocate.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use std::collections::BinaryHeap;
    /// let mut heap = BinaryHeap::with_capacity(10);
    /// heap.push(4);
    /// ```
    #[must_use]
    pub fn with_capacity(capacity: usize) -> BinaryHeap<T, C> {
        BinaryHeap {
            data: Vec::with_capacity(capacity),
            cmp: C::default(),
        }
    }
}

impl<T, F> BinaryHeap<T, FnComparator<F>>
where
    F: Fn(&T, &T) -> Ordering,
{
    #[must_use]
    pub const fn new_by(f: F) -> Self {
        BinaryHeap {
            data: vec![],
            cmp: FnComparator(f),
        }
    }
}

impl<T, C: Compare<T>> BinaryHeap<T, C> {
    /// Returns a mutable reference to the greatest item in the binary heap, or
    /// `None` if it is empty.
    ///
    /// Note: If the `PeekMut` value is leaked, some heap elements might get
    /// leaked along with it, but the remaining elements will remain a valid
    /// heap.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use std::collections::BinaryHeap;
    /// let mut heap = BinaryHeap::new();
    /// assert!(heap.peek_mut().is_none());
    ///
    /// heap.push(1);
    /// heap.push(5);
    /// heap.push(2);
    /// {
    ///     let mut val = heap.peek_mut().unwrap();
    ///     *val = 0;
    /// }
    /// assert_eq!(heap.peek(), Some(&2));
    /// ```
    ///
    /// # Time complexity
    ///
    /// If the item is modified then the worst case time complexity is *O*(log(*n*)),
    /// otherwise it's *O*(1).
    pub fn peek_mut(&mut self) -> Option<PeekMut<'_, T, C>> {
        if self.is_empty() {
            None
        } else {
            Some(PeekMut {
                heap: self,
                original_len: None,
            })
        }
    }

    /// Removes the greatest item from the binary heap and returns it, or `None` if it
    /// is empty.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use std::collections::BinaryHeap;
    /// let mut heap = BinaryHeap::from([1, 3]);
    ///
    /// assert_eq!(heap.pop(), Some(3));
    /// assert_eq!(heap.pop(), Some(1));
    /// assert_eq!(heap.pop(), None);
    /// ```
    ///
    /// # Time complexity
    ///
    /// The worst case cost of `pop` on a heap containing *n* elements is *O*(log(*n*)).
    pub fn pop(&mut self) -> Option<T> {
        self.data.pop().map(|mut item| {
            if let Some(first) = self.data.first_mut() {
                swap(&mut item, first);
                // SAFETY: self.data.first_mut() is Some(_) means that self.len() > 0
                unsafe { self.sift_down_to_bottom(0) };
            }
            item
        })
    }

    /// Pushes an item onto the binary heap.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use std::collections::BinaryHeap;
    /// let mut heap = BinaryHeap::new();
    /// heap.push(3);
    /// heap.push(5);
    /// heap.push(1);
    ///
    /// assert_eq!(heap.len(), 3);
    /// assert_eq!(heap.peek(), Some(&5));
    /// ```
    ///
    /// # Time complexity
    ///
    /// The expected cost of `push`, averaged over every possible ordering of
    /// the elements being pushed, and over a sufficiently large number of
    /// pushes, is *O*(1). This is the most meaningful cost metric when pushing
    /// elements that are *not* already in any sorted pattern.
    ///
    /// The time complexity degrades if elements are pushed in predominantly
    /// ascending order. In the worst case, elements are pushed in ascending
    /// sorted order and the amortized cost per push is *O*(log(*n*)) against a heap
    /// containing *n* elements.
    ///
    /// The worst case cost of a *single* call to `push` is *O*(*n*). The worst case
    /// occurs when capacity is exhausted and needs a resize. The resize cost
    /// has been amortized in the previous figures.
    pub fn push(&mut self, item: T) {
        let old_len = self.len();
        self.data.push(item);
        // SAFETY: Since we pushed a new item it means that
        //  old_len = self.len() - 1 < self.len()
        unsafe { self.sift_up(0, old_len) };
    }

    /// Consumes the `BinaryHeap` and returns a vector in sorted
    /// (ascending) order.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use std::collections::BinaryHeap;
    ///
    /// let mut heap = BinaryHeap::from([1, 2, 4, 5, 7]);
    /// heap.push(6);
    /// heap.push(3);
    ///
    /// let vec = heap.into_sorted_vec();
    /// assert_eq!(vec, [1, 2, 3, 4, 5, 6, 7]);
    /// ```
    #[must_use = "`self` will be dropped if the result is not used"]
    pub fn into_sorted_vec(mut self) -> Vec<T> {
        let mut end = self.len();
        while end > 1 {
            end -= 1;
            // SAFETY: `end` goes from `self.len() - 1` to 1 (both included),
            //  so it's always a valid index to access.
            //  It is safe to access index 0 (i.e. `ptr`), because
            //  1 <= end < self.len(), which means self.len() >= 2.
            unsafe {
                let ptr = self.data.as_mut_ptr();
                ptr::swap(ptr, ptr.add(end));
            }
            // SAFETY: `end` goes from `self.len() - 1` to 1 (both included) so:
            //  0 < 1 <= end <= self.len() - 1 < self.len()
            //  Which means 0 < end and end < self.len().
            unsafe { self.sift_down_range(0, end) };
        }
        self.into_vec()
    }

    // The implementations of sift_up and sift_down use unsafe blocks in
    // order to move an element out of the vector (leaving behind a
    // hole), shift along the others and move the removed element back into the
    // vector at the final location of the hole.
    // The `Hole` type is used to represent this, and make sure
    // the hole is filled back at the end of its scope, even on panic.
    // Using a hole reduces the constant factor compared to using swaps,
    // which involves twice as many moves.

    /// # Safety
    ///
    /// The caller must guarantee that `pos < self.len()`.
    unsafe fn sift_up(&mut self, start: usize, pos: usize) -> usize {
        // Take out the value at `pos` and create a hole.
        // SAFETY: The caller guarantees that pos < self.len()
        let mut hole = unsafe { Hole::new(&mut self.data, pos) };

        while hole.pos() > start {
            let parent = (hole.pos() - 1) / 2;

            // SAFETY: hole.pos() > start >= 0, which means hole.pos() > 0
            //  and so hole.pos() - 1 can't underflow.
            //  This guarantees that parent < hole.pos() so
            //  it's a valid index and also != hole.pos().
            if self
                .cmp
                .compares_le(hole.element(), unsafe { hole.get(parent) })
            {
                break;
            }

            // SAFETY: Same as above
            unsafe { hole.move_to(parent) };
        }

        hole.pos()
    }

    /// Take an element at `pos` and move it down the heap,
    /// while its children are larger.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `pos < end <= self.len()`.
    unsafe fn sift_down_range(&mut self, pos: usize, end: usize) {
        // SAFETY: The caller guarantees that pos < end <= self.len().
        let mut hole = unsafe { Hole::new(&mut self.data, pos) };
        let mut child = 2 * hole.pos() + 1;

        // Loop invariant: child == 2 * hole.pos() + 1.
        while child <= end.saturating_sub(2) {
            // compare with the greater of the two children
            // SAFETY: child < end - 1 < self.len() and
            //  child + 1 < end <= self.len(), so they're valid indexes.
            //  child == 2 * hole.pos() + 1 != hole.pos() and
            //  child + 1 == 2 * hole.pos() + 2 != hole.pos().
            // FIXME: 2 * hole.pos() + 1 or 2 * hole.pos() + 2 could overflow
            //  if T is a ZST
            child += unsafe { self.cmp.compares_le(hole.get(child), hole.get(child + 1)) } as usize;

            // if we are already in order, stop.
            // SAFETY: child is now either the old child or the old child+1
            //  We have already proven that both are < self.len() and != hole.pos()
            if self
                .cmp
                .compares_ge(hole.element(), unsafe { hole.get(child) })
            {
                return;
            }

            // SAFETY: same as above.
            unsafe { hole.move_to(child) };
            child = 2 * hole.pos() + 1;
        }

        // SAFETY: && short circuit, which means that in the
        //  second condition it's already true that child == end - 1 < self.len().
        if child == end - 1
            && self
                .cmp
                .compares_lt(hole.element(), unsafe { hole.get(child) })
        {
            // SAFETY: child is already proven to be a valid index and
            //  child == 2 * hole.pos() + 1 != hole.pos().
            unsafe { hole.move_to(child) };
        }
    }

    /// # Safety
    ///
    /// The caller must guarantee that `pos < self.len()`.
    unsafe fn sift_down(&mut self, pos: usize) {
        let len = self.len();
        // SAFETY: pos < len is guaranteed by the caller and
        //  obviously len = self.len() <= self.len().
        unsafe { self.sift_down_range(pos, len) };
    }

    /// Take an element at `pos` and move it all the way down the heap,
    /// then sift it up to its position.
    ///
    /// Note: This is faster when the element is known to be large / should
    /// be closer to the bottom.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `pos < self.len()`.
    unsafe fn sift_down_to_bottom(&mut self, mut pos: usize) {
        let end = self.len();
        let start = pos;

        // SAFETY: The caller guarantees that pos < self.len().
        let mut hole = unsafe { Hole::new(&mut self.data, pos) };
        let mut child = 2 * hole.pos() + 1;

        // Loop invariant: child == 2 * hole.pos() + 1.
        while child <= end.saturating_sub(2) {
            // SAFETY: child < end - 1 < self.len() and
            //  child + 1 < end <= self.len(), so they're valid indexes.
            //  child == 2 * hole.pos() + 1 != hole.pos() and
            //  child + 1 == 2 * hole.pos() + 2 != hole.pos().
            // FIXME: 2 * hole.pos() + 1 or 2 * hole.pos() + 2 could overflow
            //  if T is a ZST
            child += unsafe { self.cmp.compares_le(hole.get(child), hole.get(child + 1)) } as usize;

            // SAFETY: Same as above
            unsafe { hole.move_to(child) };
            child = 2 * hole.pos() + 1;
        }

        if child == end - 1 {
            // SAFETY: child == end - 1 < self.len(), so it's a valid index
            //  and child == 2 * hole.pos() + 1 != hole.pos().
            unsafe { hole.move_to(child) };
        }
        pos = hole.pos();
        drop(hole);

        // SAFETY: pos is the position in the hole and was already proven
        //  to be a valid index.
        unsafe { self.sift_up(start, pos) };
    }

    /// Rebuild assuming `data[0..start]` is still a proper heap.
    fn rebuild_tail(&mut self, start: usize) {
        if start == self.len() {
            return;
        }

        let tail_len = self.len() - start;

        #[inline(always)]
        fn log2_fast(x: usize) -> usize {
            (usize::BITS - x.leading_zeros() - 1) as usize
        }

        // `rebuild` takes O(self.len()) operations
        // and about 2 * self.len() comparisons in the worst case
        // while repeating `sift_up` takes O(tail_len * log(start)) operations
        // and about 1 * tail_len * log_2(start) comparisons in the worst case,
        // assuming start >= tail_len. For larger heaps, the crossover point
        // no longer follows this reasoning and was determined empirically.
        let better_to_rebuild = if start < tail_len {
            true
        } else if self.len() <= 2048 {
            2 * self.len() < tail_len * log2_fast(start)
        } else {
            2 * self.len() < tail_len * 11
        };

        if better_to_rebuild {
            self.rebuild();
        } else {
            for i in start..self.len() {
                // SAFETY: The index `i` is always less than self.len().
                unsafe { self.sift_up(0, i) };
            }
        }
    }

    fn rebuild(&mut self) {
        let mut n = self.len() / 2;
        while n > 0 {
            n -= 1;
            // SAFETY: n starts from self.len() / 2 and goes down to 0.
            //  The only case when !(n < self.len()) is if
            //  self.len() == 0, but it's ruled out by the loop condition.
            unsafe { self.sift_down(n) };
        }
    }

    /// Moves all the elements of `other` into `self`, leaving `other` empty.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use std::collections::BinaryHeap;
    ///
    /// let mut a = BinaryHeap::from([-10, 1, 2, 3, 3]);
    /// let mut b = BinaryHeap::from([-20, 5, 43]);
    ///
    /// a.append(&mut b);
    ///
    /// assert_eq!(a.into_sorted_vec(), [-20, -10, 1, 2, 3, 3, 5, 43]);
    /// assert!(b.is_empty());
    /// ```
    pub fn append(&mut self, other: &mut Self) {
        if self.len() < other.len() {
            swap(self, other);
        }

        let start = self.data.len();

        self.data.append(&mut other.data);

        self.rebuild_tail(start);
    }

    /// Retains only the elements specified by the predicate.
    ///
    /// In other words, remove all elements `e` for which `f(&e)` returns
    /// `false`. The elements are visited in unsorted (and unspecified) order.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use std::collections::BinaryHeap;
    ///
    /// let mut heap = BinaryHeap::from([-10, -5, 1, 2, 4, 13]);
    ///
    /// heap.retain(|x| x % 2 == 0); // only keep even numbers
    ///
    /// assert_eq!(heap.into_sorted_vec(), [-10, 2, 4])
    /// ```
    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&T) -> bool,
    {
        // rebuild_start will be updated to the first touched element below, and the rebuild will
        // only be done for the tail.
        let mut guard = RebuildOnDrop {
            rebuild_from: self.len(),
            heap: self,
        };
        let mut i = 0;

        guard.heap.data.retain(|e| {
            let keep = f(e);
            if !keep && i < guard.rebuild_from {
                guard.rebuild_from = i;
            }
            i += 1;
            keep
        });
    }
}

impl<T, C> BinaryHeap<T, C> {
    /// Returns an iterator visiting all values in the underlying vector, in
    /// arbitrary order.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use std::collections::BinaryHeap;
    /// let heap = BinaryHeap::from([1, 2, 3, 4]);
    ///
    /// // Print 1, 2, 3, 4 in arbitrary order
    /// for x in heap.iter() {
    ///     println!("{x}");
    /// }
    /// ```
    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            iter: self.data.iter(),
        }
    }

    /// Returns the greatest item in the binary heap, or `None` if it is empty.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use std::collections::BinaryHeap;
    /// let mut heap = BinaryHeap::new();
    /// assert_eq!(heap.peek(), None);
    ///
    /// heap.push(1);
    /// heap.push(5);
    /// heap.push(2);
    /// assert_eq!(heap.peek(), Some(&5));
    ///
    /// ```
    ///
    /// # Time complexity
    ///
    /// Cost is *O*(1) in the worst case.
    #[must_use]
    pub fn peek(&self) -> Option<&T> {
        self.data.first()
    }

    /// Returns the number of elements the binary heap can hold without reallocating.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use std::collections::BinaryHeap;
    /// let mut heap = BinaryHeap::with_capacity(100);
    /// assert!(heap.capacity() >= 100);
    /// heap.push(4);
    /// ```
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.data.capacity()
    }

    /// Reserves the minimum capacity for at least `additional` elements more than
    /// the current length. Unlike [`reserve`], this will not
    /// deliberately over-allocate to speculatively avoid frequent allocations.
    /// After calling `reserve_exact`, capacity will be greater than or equal to
    /// `self.len() + additional`. Does nothing if the capacity is already
    /// sufficient.
    ///
    /// [`reserve`]: BinaryHeap::reserve
    ///
    /// # Panics
    ///
    /// Panics if the new capacity overflows [`usize`].
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use std::collections::BinaryHeap;
    /// let mut heap = BinaryHeap::new();
    /// heap.reserve_exact(100);
    /// assert!(heap.capacity() >= 100);
    /// heap.push(4);
    /// ```
    ///
    /// [`reserve`]: BinaryHeap::reserve
    pub fn reserve_exact(&mut self, additional: usize) {
        self.data.reserve_exact(additional);
    }

    /// Reserves capacity for at least `additional` elements more than the
    /// current length. The allocator may reserve more space to speculatively
    /// avoid frequent allocations. After calling `reserve`,
    /// capacity will be greater than or equal to `self.len() + additional`.
    /// Does nothing if capacity is already sufficient.
    ///
    /// # Panics
    ///
    /// Panics if the new capacity overflows [`usize`].
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use std::collections::BinaryHeap;
    /// let mut heap = BinaryHeap::new();
    /// heap.reserve(100);
    /// assert!(heap.capacity() >= 100);
    /// heap.push(4);
    /// ```
    pub fn reserve(&mut self, additional: usize) {
        self.data.reserve(additional);
    }

    /// Tries to reserve the minimum capacity for at least `additional` elements
    /// more than the current length. Unlike [`try_reserve`], this will not
    /// deliberately over-allocate to speculatively avoid frequent allocations.
    /// After calling `try_reserve_exact`, capacity will be greater than or
    /// equal to `self.len() + additional` if it returns `Ok(())`.
    /// Does nothing if the capacity is already sufficient.
    ///
    /// Note that the allocator may give the collection more space than it
    /// requests. Therefore, capacity can not be relied upon to be precisely
    /// minimal. Prefer [`try_reserve`] if future insertions are expected.
    ///
    /// [`try_reserve`]: BinaryHeap::try_reserve
    ///
    /// # Errors
    ///
    /// If the capacity overflows, or the allocator reports a failure, then an error
    /// is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::BinaryHeap;
    /// use std::collections::TryReserveError;
    ///
    /// fn find_max_slow(data: &[u32]) -> Result<Option<u32>, TryReserveError> {
    ///     let mut heap = BinaryHeap::new();
    ///
    ///     // Pre-reserve the memory, exiting if we can't
    ///     heap.try_reserve_exact(data.len())?;
    ///
    ///     // Now we know this can't OOM in the middle of our complex work
    ///     heap.extend(data.iter());
    ///
    ///     Ok(heap.pop())
    /// }
    /// # find_max_slow(&[1, 2, 3]).expect("why is the test harness OOMing on 12 bytes?");
    /// ```
    pub fn try_reserve_exact(&mut self, additional: usize) -> Result<(), TryReserveError> {
        self.data.try_reserve_exact(additional)
    }

    /// Tries to reserve capacity for at least `additional` elements more than the
    /// current length. The allocator may reserve more space to speculatively
    /// avoid frequent allocations. After calling `try_reserve`, capacity will be
    /// greater than or equal to `self.len() + additional` if it returns
    /// `Ok(())`. Does nothing if capacity is already sufficient. This method
    /// preserves the contents even if an error occurs.
    ///
    /// # Errors
    ///
    /// If the capacity overflows, or the allocator reports a failure, then an error
    /// is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::BinaryHeap;
    /// use std::collections::TryReserveError;
    ///
    /// fn find_max_slow(data: &[u32]) -> Result<Option<u32>, TryReserveError> {
    ///     let mut heap = BinaryHeap::new();
    ///
    ///     // Pre-reserve the memory, exiting if we can't
    ///     heap.try_reserve(data.len())?;
    ///
    ///     // Now we know this can't OOM in the middle of our complex work
    ///     heap.extend(data.iter());
    ///
    ///     Ok(heap.pop())
    /// }
    /// # find_max_slow(&[1, 2, 3]).expect("why is the test harness OOMing on 12 bytes?");
    /// ```
    pub fn try_reserve(&mut self, additional: usize) -> Result<(), TryReserveError> {
        self.data.try_reserve(additional)
    }

    /// Discards as much additional capacity as possible.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use std::collections::BinaryHeap;
    /// let mut heap: BinaryHeap<i32> = BinaryHeap::with_capacity(100);
    ///
    /// assert!(heap.capacity() >= 100);
    /// heap.shrink_to_fit();
    /// assert_eq!(heap.capacity(), 0);
    /// ```
    pub fn shrink_to_fit(&mut self) {
        self.data.shrink_to_fit();
    }

    /// Discards capacity with a lower bound.
    ///
    /// The capacity will remain at least as large as both the length
    /// and the supplied value.
    ///
    /// If the current capacity is less than the lower limit, this is a no-op.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::BinaryHeap;
    /// let mut heap: BinaryHeap<i32> = BinaryHeap::with_capacity(100);
    ///
    /// assert!(heap.capacity() >= 100);
    /// heap.shrink_to(10);
    /// assert!(heap.capacity() >= 10);
    /// ```
    #[inline]
    pub fn shrink_to(&mut self, min_capacity: usize) {
        self.data.shrink_to(min_capacity)
    }

    /// Returns a slice of all values in the underlying vector, in arbitrary
    /// order.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use std::collections::BinaryHeap;
    /// use std::io::{self, Write};
    ///
    /// let heap = BinaryHeap::from([1, 2, 3, 4, 5, 6, 7]);
    ///
    /// io::sink().write(heap.as_slice()).unwrap();
    /// ```
    #[must_use]
    pub fn as_slice(&self) -> &[T] {
        self.data.as_slice()
    }

    /// Consumes the `BinaryHeap` and returns the underlying vector
    /// in arbitrary order.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use std::collections::BinaryHeap;
    /// let heap = BinaryHeap::from([1, 2, 3, 4, 5, 6, 7]);
    /// let vec = heap.into_vec();
    ///
    /// // Will print in some order
    /// for x in vec {
    ///     println!("{x}");
    /// }
    /// ```
    #[must_use = "`self` will be dropped if the result is not used"]
    pub fn into_vec(self) -> Vec<T> {
        self.into()
    }

    /// Returns the length of the binary heap.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use std::collections::BinaryHeap;
    /// let heap = BinaryHeap::from([1, 3]);
    ///
    /// assert_eq!(heap.len(), 2);
    /// ```
    #[must_use]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Checks if the binary heap is empty.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use std::collections::BinaryHeap;
    /// let mut heap = BinaryHeap::new();
    ///
    /// assert!(heap.is_empty());
    ///
    /// heap.push(3);
    /// heap.push(5);
    /// heap.push(1);
    ///
    /// assert!(!heap.is_empty());
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clears the binary heap, returning an iterator over the removed elements
    /// in arbitrary order. If the iterator is dropped before being fully
    /// consumed, it drops the remaining elements in arbitrary order.
    ///
    /// The returned iterator keeps a mutable borrow on the heap to optimize
    /// its implementation.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use std::collections::BinaryHeap;
    /// let mut heap = BinaryHeap::from([1, 3]);
    ///
    /// assert!(!heap.is_empty());
    ///
    /// for x in heap.drain() {
    ///     println!("{x}");
    /// }
    ///
    /// assert!(heap.is_empty());
    /// ```
    #[inline]
    pub fn drain(&mut self) -> Drain<'_, T> {
        Drain {
            iter: self.data.drain(..),
        }
    }

    /// Drops all items from the binary heap.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use std::collections::BinaryHeap;
    /// let mut heap = BinaryHeap::from([1, 3]);
    ///
    /// assert!(!heap.is_empty());
    ///
    /// heap.clear();
    ///
    /// assert!(heap.is_empty());
    /// ```
    pub fn clear(&mut self) {
        self.drain();
    }
}

/// Hole represents a hole in a slice i.e., an index without valid value
/// (because it was moved from or duplicated).
/// In drop, `Hole` will restore the slice by filling the hole
/// position with the value that was originally removed.
struct Hole<'a, T: 'a> {
    data: &'a mut [T],
    elt: ManuallyDrop<T>,
    pos: usize,
}

impl<'a, T> Hole<'a, T> {
    /// Create a new `Hole` at index `pos`.
    ///
    /// Unsafe because pos must be within the data slice.
    #[inline]
    unsafe fn new(data: &'a mut [T], pos: usize) -> Self {
        debug_assert!(pos < data.len());
        // SAFE: pos should be inside the slice
        let elt = unsafe { ptr::read(data.get_unchecked(pos)) };
        Hole {
            data,
            elt: ManuallyDrop::new(elt),
            pos,
        }
    }

    #[inline]
    fn pos(&self) -> usize {
        self.pos
    }

    /// Returns a reference to the element removed.
    #[inline]
    fn element(&self) -> &T {
        &self.elt
    }

    /// Returns a reference to the element at `index`.
    ///
    /// Unsafe because index must be within the data slice and not equal to pos.
    #[inline]
    unsafe fn get(&self, index: usize) -> &T {
        debug_assert!(index != self.pos);
        debug_assert!(index < self.data.len());
        unsafe { self.data.get_unchecked(index) }
    }

    /// Move hole to new location
    ///
    /// Unsafe because index must be within the data slice and not equal to pos.
    #[inline]
    unsafe fn move_to(&mut self, index: usize) {
        debug_assert!(index != self.pos);
        debug_assert!(index < self.data.len());
        unsafe {
            let ptr = self.data.as_mut_ptr();
            let index_ptr: *const _ = ptr.add(index);
            let hole_ptr = ptr.add(self.pos);
            ptr::copy_nonoverlapping(index_ptr, hole_ptr, 1);
        }
        self.pos = index;
    }
}

impl<T> Drop for Hole<'_, T> {
    #[inline]
    fn drop(&mut self) {
        // fill the hole again
        unsafe {
            let pos = self.pos;
            ptr::copy_nonoverlapping(&*self.elt, self.data.get_unchecked_mut(pos), 1);
        }
    }
}

/// An iterator over the elements of a `BinaryHeap`.
///
/// This `struct` is created by [`BinaryHeap::iter()`]. See its
/// documentation for more.
///
/// [`iter`]: BinaryHeap::iter
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct Iter<'a, T: 'a> {
    iter: slice::Iter<'a, T>,
}

impl<T: fmt::Debug> fmt::Debug for Iter<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Iter").field(&self.iter.as_slice()).finish()
    }
}

// FIXME(#26925) Remove in favor of `#[derive(Clone)]`
impl<T> Clone for Iter<'_, T> {
    fn clone(&self) -> Self {
        Iter {
            iter: self.iter.clone(),
        }
    }
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    #[inline]
    fn next(&mut self) -> Option<&'a T> {
        self.iter.next()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }

    #[inline]
    fn last(self) -> Option<&'a T> {
        self.iter.last()
    }
}

impl<'a, T> DoubleEndedIterator for Iter<'a, T> {
    #[inline]
    fn next_back(&mut self) -> Option<&'a T> {
        self.iter.next_back()
    }
}

impl<T> FusedIterator for Iter<'_, T> {}

/// An owning iterator over the elements of a `BinaryHeap`.
///
/// This `struct` is created by [`BinaryHeap::into_iter()`]
/// (provided by the [`IntoIterator`] trait). See its documentation for more.
///
/// [`into_iter`]: BinaryHeap::into_iter
#[derive(Clone)]
pub struct IntoIter<T> {
    iter: vec::IntoIter<T>,
}

impl<T: fmt::Debug> fmt::Debug for IntoIter<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("IntoIter")
            .field(&self.iter.as_slice())
            .finish()
    }
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<T> {
        self.iter.next()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<T> DoubleEndedIterator for IntoIter<T> {
    #[inline]
    fn next_back(&mut self) -> Option<T> {
        self.iter.next_back()
    }
}

impl<T> FusedIterator for IntoIter<T> {}

impl<T> Default for IntoIter<T> {
    /// Creates an empty `binary_heap::IntoIter`.
    ///
    /// ```
    /// # use std::collections::binary_heap;
    /// let iter: binary_heap::IntoIter<u8> = Default::default();
    /// assert_eq!(iter.len(), 0);
    /// ```
    fn default() -> Self {
        IntoIter {
            iter: Default::default(),
        }
    }
}

/// A draining iterator over the elements of a `BinaryHeap`.
///
/// This `struct` is created by [`BinaryHeap::drain()`]. See its
/// documentation for more.
///
/// [`drain`]: BinaryHeap::drain
#[derive(Debug)]
pub struct Drain<'a, T: 'a> {
    iter: vec::Drain<'a, T>,
}

impl<T> Iterator for Drain<'_, T> {
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<T> {
        self.iter.next()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<T> DoubleEndedIterator for Drain<'_, T> {
    #[inline]
    fn next_back(&mut self) -> Option<T> {
        self.iter.next_back()
    }
}

impl<T> FusedIterator for Drain<'_, T> {}

impl<T, C> From<BinaryHeap<T, C>> for Vec<T> {
    /// Converts a `BinaryHeap<T>` into a `Vec<T>`.
    ///
    /// This conversion requires no data movement or allocation, and has
    /// constant time complexity.
    fn from(heap: BinaryHeap<T, C>) -> Vec<T> {
        heap.data
    }
}

impl<T, C> IntoIterator for BinaryHeap<T, C> {
    type Item = T;
    type IntoIter = IntoIter<T>;

    /// Creates a consuming iterator, that is, one that moves each value out of
    /// the binary heap in arbitrary order. The binary heap cannot be used
    /// after calling this.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use std::collections::BinaryHeap;
    /// let heap = BinaryHeap::from([1, 2, 3, 4]);
    ///
    /// // Print 1, 2, 3, 4 in arbitrary order
    /// for x in heap.into_iter() {
    ///     // x has type i32, not &i32
    ///     println!("{x}");
    /// }
    /// ```
    fn into_iter(self) -> IntoIter<T> {
        IntoIter {
            iter: self.data.into_iter(),
        }
    }
}

impl<'a, T, C> IntoIterator for &'a BinaryHeap<T, C> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Iter<'a, T> {
        self.iter()
    }
}

impl<T, C: Compare<T>> Extend<T> for BinaryHeap<T, C> {
    #[inline]
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        let guard = RebuildOnDrop {
            rebuild_from: self.len(),
            heap: self,
        };
        guard.heap.data.extend(iter);
    }
}

impl<'a, T: 'a + Copy, C: 'a + Compare<T>> Extend<&'a T> for BinaryHeap<T, C> {
    fn extend<I: IntoIterator<Item = &'a T>>(&mut self, iter: I) {
        self.extend(iter.into_iter().cloned());
    }
}

impl<T, C: Compare<T> + Default> From<Vec<T>> for BinaryHeap<T, C> {
    /// Converts a `Vec<T>` into a `BinaryHeap<T>`.
    ///
    /// This conversion happens in-place, and has *O*(*n*) time complexity.
    fn from(vec: Vec<T>) -> BinaryHeap<T, C> {
        let mut heap = BinaryHeap {
            data: vec,
            cmp: C::default(),
        };
        heap.rebuild();
        heap
    }
}

impl<T, C: Compare<T> + Default, const N: usize> From<[T; N]> for BinaryHeap<T, C> {
    fn from(arr: [T; N]) -> Self {
        Self::from_iter(arr)
    }
}

impl<T, C: Compare<T> + Default> FromIterator<T> for BinaryHeap<T, C> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> BinaryHeap<T, C> {
        BinaryHeap::from(iter.into_iter().collect::<Vec<_>>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    #[test]
    fn test_iterator() {
        let data = vec![5, 9, 3];
        let iterout = [9, 5, 3];
        let heap = BinaryHeap::<_, MaxComparator>::from(data);
        for (i, el) in iterout.iter().zip(&heap) {
            assert_eq!(*el, *i);
        }
    }

    #[test]
    fn test_iter_rev_cloned_collect() {
        let data = vec![5, 9, 3];
        let iterout = vec![3, 5, 9];
        let pq = BinaryHeap::<_, MaxComparator>::from(data);

        let v: Vec<_> = pq.iter().rev().cloned().collect();
        assert_eq!(v, iterout);
    }

    #[test]
    fn test_into_iter_collect() {
        let data = vec![5, 9, 3];
        let iterout = vec![9, 5, 3];
        let pq = BinaryHeap::<_, MaxComparator>::from(data);

        let v: Vec<_> = pq.into_iter().collect();
        assert_eq!(v, iterout);
    }

    #[test]
    fn test_into_iter_size_hint() {
        let data = vec![5, 9];
        let pq = BinaryHeap::<_, MaxComparator>::from(data);

        let mut it = pq.into_iter();

        assert_eq!(it.size_hint(), (2, Some(2)));
        assert_eq!(it.next(), Some(9));

        assert_eq!(it.size_hint(), (1, Some(1)));
        assert_eq!(it.next(), Some(5));

        assert_eq!(it.size_hint(), (0, Some(0)));
        assert_eq!(it.next(), None);
    }

    #[test]
    fn test_into_iter_rev_collect() {
        let data = vec![5, 9, 3];
        let iterout = vec![3, 5, 9];
        let pq = BinaryHeap::<_, MaxComparator>::from(data);

        let v: Vec<_> = pq.into_iter().rev().collect();
        assert_eq!(v, iterout);
    }

    #[test]
    fn test_peek_and_pop() {
        let data = vec![2, 4, 6, 2, 1, 8, 10, 3, 5, 7, 0, 9, 1];
        let mut sorted = data.clone();
        sorted.sort();
        let mut heap = BinaryHeap::<_, MaxComparator>::from(data);
        while !heap.is_empty() {
            assert_eq!(heap.peek().unwrap(), sorted.last().unwrap());
            assert_eq!(heap.pop().unwrap(), sorted.pop().unwrap());
        }
    }

    #[test]
    fn test_peek_mut() {
        let data = vec![2, 4, 6, 2, 1, 8, 10, 3, 5, 7, 0, 9, 1];
        let mut heap = BinaryHeap::<_, MaxComparator>::from(data);
        assert_eq!(heap.peek(), Some(&10));
        {
            let mut top = heap.peek_mut().unwrap();
            *top -= 2;
        }
        assert_eq!(heap.peek(), Some(&9));
    }

    #[test]
    fn test_peek_mut_pop() {
        let data = vec![2, 4, 6, 2, 1, 8, 10, 3, 5, 7, 0, 9, 1];
        let mut heap = BinaryHeap::<_, MaxComparator>::from(data);
        assert_eq!(heap.peek(), Some(&10));
        {
            let mut top = heap.peek_mut().unwrap();
            *top -= 2;
            assert_eq!(PeekMut::pop(top), 8);
        }
        assert_eq!(heap.peek(), Some(&9));
    }

    #[test]
    fn test_push() {
        let mut heap = BinaryHeap::<_, MaxComparator>::from(vec![2, 4, 9]);
        assert_eq!(heap.len(), 3);
        assert_eq!(*heap.peek().unwrap(), 9);
        heap.push(11);
        assert_eq!(heap.len(), 4);
        assert_eq!(*heap.peek().unwrap(), 11);
        heap.push(5);
        assert_eq!(heap.len(), 5);
        assert_eq!(*heap.peek().unwrap(), 11);
        heap.push(27);
        assert_eq!(heap.len(), 6);
        assert_eq!(*heap.peek().unwrap(), 27);
        heap.push(3);
        assert_eq!(heap.len(), 7);
        assert_eq!(*heap.peek().unwrap(), 27);
        heap.push(103);
        assert_eq!(heap.len(), 8);
        assert_eq!(*heap.peek().unwrap(), 103);
    }

    #[test]
    fn test_push_unique() {
        let mut heap =
            BinaryHeap::<Box<_>, MaxComparator>::from(vec![Box::new(2), Box::new(4), Box::new(9)]);
        assert_eq!(heap.len(), 3);
        assert_eq!(**heap.peek().unwrap(), 9);
        heap.push(Box::new(11));
        assert_eq!(heap.len(), 4);
        assert_eq!(**heap.peek().unwrap(), 11);
        heap.push(Box::new(5));
        assert_eq!(heap.len(), 5);
        assert_eq!(**heap.peek().unwrap(), 11);
        heap.push(Box::new(27));
        assert_eq!(heap.len(), 6);
        assert_eq!(**heap.peek().unwrap(), 27);
        heap.push(Box::new(3));
        assert_eq!(heap.len(), 7);
        assert_eq!(**heap.peek().unwrap(), 27);
        heap.push(Box::new(103));
        assert_eq!(heap.len(), 8);
        assert_eq!(**heap.peek().unwrap(), 103);
    }

    fn check_to_vec(mut data: Vec<i32>) {
        let heap = BinaryHeap::<_, MaxComparator>::from(data.clone());
        let mut v = heap.clone().into_vec();
        v.sort();
        data.sort();

        assert_eq!(v, data);
        assert_eq!(heap.into_sorted_vec(), data);
    }

    #[test]
    fn test_to_vec() {
        check_to_vec(vec![]);
        check_to_vec(vec![5]);
        check_to_vec(vec![3, 2]);
        check_to_vec(vec![2, 3]);
        check_to_vec(vec![5, 1, 2]);
        check_to_vec(vec![1, 100, 2, 3]);
        check_to_vec(vec![1, 3, 5, 7, 9, 2, 4, 6, 8, 0]);
        check_to_vec(vec![2, 4, 6, 2, 1, 8, 10, 3, 5, 7, 0, 9, 1]);
        check_to_vec(vec![9, 11, 9, 9, 9, 9, 11, 2, 3, 4, 11, 9, 0, 0, 0, 0]);
        check_to_vec(vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        check_to_vec(vec![10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0]);
        check_to_vec(vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 0, 0, 1, 2]);
        check_to_vec(vec![5, 4, 3, 2, 1, 5, 4, 3, 2, 1, 5, 4, 3, 2, 1]);
    }

    #[test]
    fn test_empty_pop() {
        let mut heap = BinaryHeap::<i32, MaxComparator>::new();
        assert!(heap.pop().is_none());
    }

    #[test]
    fn test_empty_peek() {
        let empty = BinaryHeap::<i32, MaxComparator>::new();
        assert!(empty.peek().is_none());
    }

    #[test]
    fn test_empty_peek_mut() {
        let mut empty = BinaryHeap::<i32, MaxComparator>::new();
        assert!(empty.peek_mut().is_none());
    }

    #[test]
    fn test_from_iter() {
        let xs = vec![9, 8, 7, 6, 5, 4, 3, 2, 1];

        let mut q: BinaryHeap<_, MaxComparator> = xs.iter().rev().cloned().collect();

        for &x in &xs {
            assert_eq!(q.pop().unwrap(), x);
        }
    }

    #[test]
    fn test_drain() {
        let mut q: BinaryHeap<_, MaxComparator> =
            [9, 8, 7, 6, 5, 4, 3, 2, 1].iter().cloned().collect();

        assert_eq!(q.drain().take(5).count(), 5);

        assert!(q.is_empty());
    }

    #[test]
    fn test_extend_ref() {
        let mut a = BinaryHeap::<_, MaxComparator>::new();
        a.push(1);
        a.push(2);

        a.extend(&[3, 4, 5]);

        assert_eq!(a.len(), 5);
        assert_eq!(a.into_sorted_vec(), [1, 2, 3, 4, 5]);

        let mut a = BinaryHeap::<_, MaxComparator>::new();
        a.push(1);
        a.push(2);
        let mut b = BinaryHeap::<_, MaxComparator>::new();
        b.push(3);
        b.push(4);
        b.push(5);

        a.extend(&b);

        assert_eq!(a.len(), 5);
        assert_eq!(a.into_sorted_vec(), [1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_append() {
        let mut a = BinaryHeap::<_, MaxComparator>::from(vec![-10, 1, 2, 3, 3]);
        let mut b = BinaryHeap::<_, MaxComparator>::from(vec![-20, 5, 43]);

        a.append(&mut b);

        assert_eq!(a.into_sorted_vec(), [-20, -10, 1, 2, 3, 3, 5, 43]);
        assert!(b.is_empty());
    }

    #[test]
    fn test_append_to_empty() {
        let mut a = BinaryHeap::<_, MaxComparator>::new();
        let mut b = BinaryHeap::from(vec![-20, 5, 43]);

        a.append(&mut b);

        assert_eq!(a.into_sorted_vec(), [-20, 5, 43]);
        assert!(b.is_empty());
    }

    #[test]
    fn test_extend_specialization() {
        let mut a = BinaryHeap::<_, MaxComparator>::from(vec![-10, 1, 2, 3, 3]);
        let b = BinaryHeap::<_, MaxComparator>::from(vec![-20, 5, 43]);

        a.extend(b);

        assert_eq!(a.into_sorted_vec(), [-20, -10, 1, 2, 3, 3, 5, 43]);
    }

    #[allow(dead_code)]
    fn assert_covariance() {
        fn drain<'new>(d: Drain<'static, &'static str>) -> Drain<'new, &'new str> {
            d
        }
    }

    #[test]
    fn test_retain() {
        let mut a = BinaryHeap::<_, MaxComparator>::from(vec![100, 10, 50, 1, 2, 20, 30]);
        a.retain(|&x| x != 2);

        // Check that 20 moved into 10's place.
        assert_eq!(a.clone().into_vec(), [100, 20, 50, 1, 10, 30]);

        a.retain(|_| true);

        assert_eq!(a.clone().into_vec(), [100, 20, 50, 1, 10, 30]);

        a.retain(|&x| x < 50);

        assert_eq!(a.clone().into_vec(), [30, 20, 10, 1]);

        a.retain(|_| false);

        assert!(a.is_empty());
    }

    #[test]
    #[cfg_attr(not(panic = "unwind"), ignore = "test requires unwinding support")]
    fn test_retain_catch_unwind() {
        let mut heap = BinaryHeap::<_, MaxComparator>::from(vec![3, 1, 2]);

        // Removes the 3, then unwinds out of retain.
        let _ = catch_unwind(AssertUnwindSafe(|| {
            heap.retain(|e| {
                if *e == 1 {
                    panic!();
                }
                false
            });
        }));

        // Naively this would be [1, 2] (an invalid heap) if BinaryHeap delegates to
        // Vec's retain impl and then does not rebuild the heap after that unwinds.
        assert_eq!(heap.into_vec(), [2, 1]);
    }
}
