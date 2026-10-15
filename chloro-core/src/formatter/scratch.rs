//! Scratch storage for the formatter's short-lived intermediates (EXP-025): a bump arena per
//! thread, and [`ScratchVec`] and [`ScratchString`], a `Vec` and a `String` whose storage
//! comes from it instead of the global allocator.
//!
//! Not a rustfmt module: rustfmt's modules allocate every intermediate with the global
//! allocator. These types hold only values that are made and dropped inside one module (a
//! call's list items, a chain's entries, a path segment's text); a value that leaves its
//! module stays a `String`, so the `Option<String>` protocol between modules is unchanged.
//!
//! The arena:
//! - an allocation bumps a pointer in the current chunk (16 KiB first, doubling up to
//!   1 MiB); a new chunk comes from the global allocator when the current one is full;
//! - the newest allocation grows in place, and its space is reused when it is the first of
//!   the live allocations to be dropped;
//! - when the last scratch value that holds storage is dropped, the arena resets to the start
//!   of its newest chunk and frees the others (the newest too when above 1 MiB). Scratch
//!   values live inside one rewrite of one top-level item, so the arena is empty at every
//!   item boundary at the latest: its memory is bounded by the scratch of one item, not of
//!   the file.
//!
//! Safety: a byte of a chunk is handed out at most once between two resets, and a reset (or a
//! chunk's release) happens only when no scratch value holds storage: each value that holds
//! storage counts one in `live` from its first allocation to its drop. The values are neither
//! `Send` nor `Sync`, so they never reach another thread or its arena. A value forgotten with
//! `mem::forget` keeps `live` above zero, which stops resets (the arena then only grows) but
//! never frees storage in use.

use std::alloc::{Layout, alloc, dealloc, handle_alloc_error};
use std::cell::{Cell, RefCell};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::ptr::{self, NonNull};
use std::{fmt, mem, slice, str};

const FIRST_CHUNK: usize = 16 << 10;
const KEEP_LIMIT: usize = 1 << 20;
const CHUNK_ALIGN: usize = 16;

struct Arena {
    /// The next free byte of the current chunk, its start and its end (all null before the
    /// first chunk). Pointers, not addresses, so that every pointer handed out keeps its
    /// chunk's provenance.
    ptr: Cell<*mut u8>,
    start: Cell<*mut u8>,
    end: Cell<*mut u8>,
    /// Scratch values that hold storage.
    live: Cell<usize>,
    /// Chunks other than the current one, which a reset frees.
    older: Cell<bool>,
    /// Every chunk as (start, size), the current one last.
    chunks: RefCell<Vec<(*mut u8, usize)>>,
}

thread_local! {
    static ARENA: Arena = const {
        Arena {
            ptr: Cell::new(ptr::null_mut()),
            start: Cell::new(ptr::null_mut()),
            end: Cell::new(ptr::null_mut()),
            live: Cell::new(0),
            older: Cell::new(false),
            chunks: RefCell::new(Vec::new()),
        }
    };
}

impl Arena {
    /// `size` bytes at `align` (a power of two, at most `CHUNK_ALIGN`); `size > 0`.
    #[inline(always)]
    fn alloc(&self, size: usize, align: usize) -> *mut u8 {
        let top = self.ptr.get();
        let p = (top.addr() + (align - 1)) & !(align - 1);
        let end = self.end.get().addr();
        // Before the first chunk `top` and `end` are null and the test fails (`size > 0`).
        if p <= end && size <= end - p {
            // The current chunk's provenance at an address inside it (`p + size <= end`).
            let p = top.with_addr(p);
            self.ptr.set(p.wrapping_add(size));
            return p;
        }
        self.alloc_in_new_chunk(size, align)
    }

    #[cold]
    #[inline(never)]
    fn alloc_in_new_chunk(&self, size: usize, align: usize) -> *mut u8 {
        let mut chunks = self.chunks.borrow_mut();
        let last = chunks.last().map_or(0, |&(_, s)| s);
        let want = size
            .checked_add(align)
            .expect("scratch allocation too large");
        let chunk = (last * 2).clamp(FIRST_CHUNK, KEEP_LIMIT).max(want);
        let layout = Layout::from_size_align(chunk, CHUNK_ALIGN).expect("chunk layout");
        // SAFETY: `layout` has a non-zero size.
        let base = unsafe { alloc(layout) };
        if base.is_null() {
            handle_alloc_error(layout);
        }
        self.older.set(!chunks.is_empty());
        chunks.push((base, chunk));
        // SAFETY: `base` starts an allocation of `chunk >= size + align` bytes aligned to
        // `CHUNK_ALIGN >= align`, so `base` is aligned and `base + size` is inside it.
        unsafe {
            self.start.set(base);
            self.end.set(base.add(chunk));
            self.ptr.set(base.add(size));
        }
        base
    }

    /// Grows the allocation at `p` of `old` bytes to `new > old` bytes: in place when it is
    /// the newest allocation of the current chunk and the chunk has room, else by a new
    /// allocation and a copy of the `old` bytes (the old space is reused after the next reset).
    #[inline]
    fn grow(&self, p: *mut u8, old: usize, new: usize, align: usize) -> *mut u8 {
        let top = self.ptr.get();
        if p.addr() + old == top.addr()
            && p.addr() >= self.start.get().addr()
            && new - old <= self.end.get().addr() - top.addr()
        {
            // SAFETY: `p` is the newest allocation of the current chunk (it ends at `top`)
            // and the chunk has `new - old` bytes after `top`.
            self.ptr.set(unsafe { top.add(new - old) });
            return p;
        }
        let q = self.alloc(new, align);
        // SAFETY: `p` holds `old` initialised-or-not bytes owned by the caller; `q` is a fresh
        // allocation of `new > old` bytes, so the two do not overlap.
        unsafe { ptr::copy_nonoverlapping(p, q, old) };
        q
    }

    /// The storage of a value that is dropped: given back when it is the newest allocation;
    /// the arena resets when no value holds storage any more.
    #[inline(always)]
    fn release(&self, p: *mut u8, size: usize) {
        if p.addr() + size == self.ptr.get().addr() && p.addr() >= self.start.get().addr() {
            self.ptr.set(p);
        }
        let live = self.live.get() - 1;
        self.live.set(live);
        if live == 0 {
            if self.older.get() || self.end.get().addr() - self.start.get().addr() > KEEP_LIMIT {
                self.free_chunks();
            } else {
                self.ptr.set(self.start.get());
            }
        }
    }

    /// Frees every chunk but the current one (and that one too when it is above
    /// `KEEP_LIMIT`). Only when `live` is zero.
    #[cold]
    #[inline(never)]
    fn free_chunks(&self) {
        debug_assert_eq!(self.live.get(), 0);
        let mut chunks = self.chunks.borrow_mut();
        let keep = match chunks.last() {
            Some(&(_, size)) if size <= KEEP_LIMIT => chunks.pop(),
            _ => None,
        };
        for (base, size) in chunks.drain(..) {
            // SAFETY: allocated in `alloc_in_new_chunk` with this layout; no value holds
            // storage in it (`live` is zero).
            unsafe { dealloc(base, Layout::from_size_align_unchecked(size, CHUNK_ALIGN)) };
        }
        let (base, end) = match keep {
            // SAFETY: `base + size` is the end of the kept chunk.
            Some((base, size)) => (base, unsafe { base.add(size) }),
            None => (ptr::null_mut(), ptr::null_mut()),
        };
        if let Some(k) = keep {
            chunks.push(k);
        }
        self.older.set(false);
        self.start.set(base);
        self.ptr.set(base);
        self.end.set(end);
    }
}

impl Drop for Arena {
    fn drop(&mut self) {
        // A value still holding storage (forgotten, or held by another thread-local) keeps
        // the chunks: they are leaked rather than freed under it.
        if self.live.get() == 0 {
            for &(base, size) in self.chunks.get_mut().iter() {
                // SAFETY: as in `free_chunks`.
                unsafe { dealloc(base, Layout::from_size_align_unchecked(size, CHUNK_ALIGN)) };
            }
        }
    }
}

/// A `Vec<T>` whose storage is in the thread's scratch arena. For values made and dropped
/// inside one module; see the module documentation.
pub(crate) struct ScratchVec<T> {
    ptr: NonNull<T>,
    len: usize,
    cap: usize,
    /// Owns `T`s; `*mut u8` makes the type neither `Send` nor `Sync`.
    _marker: PhantomData<(T, *mut u8)>,
}

impl<T> ScratchVec<T> {
    const SUPPORTED: () = assert!(
        mem::size_of::<T>() != 0 && mem::align_of::<T>() <= CHUNK_ALIGN,
        "ScratchVec holds sized, non-zero-sized types aligned to at most 16 bytes"
    );

    pub(crate) const fn new() -> Self {
        let () = Self::SUPPORTED;
        ScratchVec {
            ptr: NonNull::dangling(),
            len: 0,
            cap: 0,
            _marker: PhantomData,
        }
    }

    #[inline(always)]
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        let mut v = Self::new();
        if capacity > 0 {
            v.first_alloc(capacity);
        }
        v
    }

    #[inline]
    pub(crate) fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub(crate) fn push(&mut self, value: T) {
        if self.len == self.cap {
            self.grow_for(1);
        }
        // SAFETY: `len < cap`, so the slot is inside the storage and uninitialised.
        unsafe { self.ptr.as_ptr().add(self.len).write(value) };
        self.len += 1;
    }

    pub(crate) fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        // SAFETY: the slot at the old `len - 1` is initialised and is no longer counted.
        Some(unsafe { self.ptr.as_ptr().add(self.len).read() })
    }

    pub(crate) fn reserve(&mut self, additional: usize) {
        if additional > self.cap - self.len {
            self.grow_for(additional);
        }
    }

    #[cold]
    #[inline(never)]
    fn grow_for(&mut self, additional: usize) {
        let need = self.len.checked_add(additional).expect("capacity overflow");
        let cap = need.max(self.cap * 2).max(4);
        if self.cap == 0 {
            self.first_alloc(cap);
            return;
        }
        let size = mem::size_of::<T>();
        let bytes = cap.checked_mul(size).expect("capacity overflow");
        let old = self.ptr.as_ptr().cast();
        let p = ARENA.with(|a| a.grow(old, self.cap * size, bytes, mem::align_of::<T>()));
        // SAFETY: `grow` never returns null.
        self.ptr = unsafe { NonNull::new_unchecked(p.cast()) };
        self.cap = cap;
    }

    /// The first storage, for `cap > 0` values; `self.cap` is zero.
    #[inline(always)]
    fn first_alloc(&mut self, cap: usize) {
        let bytes = cap
            .checked_mul(mem::size_of::<T>())
            .expect("capacity overflow");
        let p = ARENA.with(|a| {
            a.live.set(a.live.get() + 1);
            a.alloc(bytes, mem::align_of::<T>())
        });
        // SAFETY: `alloc` never returns null.
        self.ptr = unsafe { NonNull::new_unchecked(p.cast()) };
        self.cap = cap;
    }
}

impl<T> Drop for ScratchVec<T> {
    fn drop(&mut self) {
        // SAFETY: the first `len` slots are initialised and owned.
        unsafe { ptr::drop_in_place(ptr::slice_from_raw_parts_mut(self.ptr.as_ptr(), self.len)) };
        if self.cap != 0 {
            let (p, bytes) = (self.ptr.as_ptr().cast(), self.cap * mem::size_of::<T>());
            // After the thread's arena is gone the storage is leaked with it.
            let _ = ARENA.try_with(|a| a.release(p, bytes));
        }
    }
}

impl<T> Default for ScratchVec<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Deref for ScratchVec<T> {
    type Target = [T];
    #[inline]
    fn deref(&self) -> &[T] {
        // SAFETY: `ptr` is non-null and aligned (dangling when `cap == 0`, with `len == 0`);
        // the first `len` slots are initialised.
        unsafe { slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }
}

impl<T> DerefMut for ScratchVec<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut [T] {
        // SAFETY: as in `deref`, and `&mut self` is unique.
        unsafe { slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
    }
}

impl<T> Extend<T> for ScratchVec<T> {
    /// As `Vec`'s: an empty vector takes room for at least four values once there is one.
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        let mut iter = iter.into_iter();
        if self.cap == 0 {
            let Some(first) = iter.next() else {
                return;
            };
            self.first_alloc(iter.size_hint().0.saturating_add(1).max(4));
            self.push(first);
        } else {
            self.reserve(iter.size_hint().0);
        }
        for x in iter {
            self.push(x);
        }
    }
}

impl<T> FromIterator<T> for ScratchVec<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut v = Self::new();
        v.extend(iter);
        v
    }
}

impl<'v, T> IntoIterator for &'v ScratchVec<T> {
    type Item = &'v T;
    type IntoIter = slice::Iter<'v, T>;
    fn into_iter(self) -> slice::Iter<'v, T> {
        self.iter()
    }
}

impl<T: fmt::Debug> fmt::Debug for ScratchVec<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

/// A `String` whose storage is in the thread's scratch arena; see [`ScratchVec`].
#[derive(Default)]
pub(crate) struct ScratchString {
    bytes: ScratchVec<u8>,
}

impl ScratchString {
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        ScratchString {
            bytes: ScratchVec::with_capacity(capacity),
        }
    }

    #[inline]
    pub(crate) fn push_str(&mut self, s: &str) {
        self.bytes.reserve(s.len());
        // SAFETY: `reserve` made room for `s.len()` bytes after `len`; they do not overlap
        // `s`, which is not in this value's storage (`&mut self` is unique).
        unsafe {
            ptr::copy_nonoverlapping(
                s.as_ptr(),
                self.bytes.ptr.as_ptr().add(self.bytes.len),
                s.len(),
            );
        }
        self.bytes.len += s.len();
    }

    pub(crate) fn push(&mut self, c: char) {
        self.push_str(c.encode_utf8(&mut [0; 4]));
    }

    pub(crate) fn clear(&mut self) {
        self.bytes.len = 0;
    }

    #[inline]
    pub(crate) fn as_str(&self) -> &str {
        // SAFETY: only whole `str`s are ever appended.
        unsafe { str::from_utf8_unchecked(&self.bytes) }
    }
}

impl Deref for ScratchString {
    type Target = str;
    #[inline]
    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Write for ScratchString {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.push_str(s);
        Ok(())
    }
}

impl fmt::Debug for ScratchString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl fmt::Display for ScratchString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    fn arena_state() -> (usize, usize, usize, usize) {
        ARENA.with(|a| {
            (
                a.ptr.get().addr(),
                a.start.get().addr(),
                a.live.get(),
                a.chunks.borrow().len(),
            )
        })
    }

    #[test]
    fn vec_matches_std_vec_through_pushes_pops_and_growth() {
        let mut ours = ScratchVec::new();
        let mut std = Vec::new();
        // Interleaved with a second vector so that growth is sometimes in place and
        // sometimes a move.
        let mut other = ScratchVec::with_capacity(1);
        for i in 0..5000u64 {
            ours.push(i * 7);
            std.push(i * 7);
            if i % 3 == 0 {
                other.push(i as u8);
            }
            if i % 11 == 0 {
                assert_eq!(ours.pop(), std.pop());
            }
            assert_eq!(&*ours, &std[..]);
        }
        ours.extend(std.iter().copied().take(100));
        std.extend(std.clone().into_iter().take(100));
        assert_eq!(&*ours, &std[..]);
        let collected: ScratchVec<u64> = std.iter().map(|x| x + 1).collect();
        assert!(collected.iter().zip(&std).all(|(a, b)| *a == b + 1));
        assert_eq!(other.len(), 1667);
    }

    #[test]
    fn newest_grows_in_place_and_is_given_back() {
        let (_, _, live0, _) = arena_state();
        assert_eq!(live0, 0, "no scratch value outlives a test");
        let mut a = ScratchVec::<u32>::with_capacity(4);
        let first = a.ptr.as_ptr();
        for i in 0..64 {
            a.push(i);
        }
        assert_eq!(
            a.ptr.as_ptr(),
            first,
            "the newest allocation grows in place"
        );
        let b = ScratchVec::<u32>::with_capacity(4);
        let top_with_b = arena_state().0;
        drop(b);
        assert!(
            arena_state().0 < top_with_b,
            "the newest allocation is given back"
        );
        let mut c = ScratchVec::<u32>::with_capacity(4);
        c.push(1);
        for i in 64..200 {
            a.push(i);
        }
        assert_ne!(
            a.ptr.as_ptr(),
            first,
            "an older allocation moves when it grows"
        );
        assert_eq!(&a[..], &(0..200).collect::<Vec<_>>()[..]);
        drop(a);
        drop(c);
        let (ptr, start, live, chunks) = arena_state();
        assert_eq!(
            (live, ptr, chunks),
            (0, start, 1),
            "reset when the last value is dropped"
        );
    }

    #[test]
    fn large_values_take_new_chunks_and_a_reset_frees_them() {
        let mut held = Vec::new();
        for i in 0..40 {
            let mut s = ScratchString::with_capacity(10);
            for _ in 0..(1000 * (i % 5 + 1)) {
                s.push_str("abc");
            }
            assert_eq!(s.len(), 3000 * (i % 5 + 1));
            held.push(s);
        }
        let big = ScratchVec::<u8>::with_capacity(3 << 20);
        assert!(arena_state().3 > 2);
        assert!(
            held.iter()
                .all(|s| s.bytes.iter().all(|&b| b"abc".contains(&b)))
        );
        drop(held);
        drop(big);
        let (ptr, start, live, chunks) = arena_state();
        assert_eq!(
            (live, ptr, chunks),
            (0, start, 0),
            "a chunk above 1 MiB is not kept"
        );
        let s = ScratchString::with_capacity(1);
        assert_eq!(arena_state().3, 1);
        drop(s);
    }

    #[test]
    fn elements_are_dropped_once() {
        let rc = Rc::new(());
        let mut v = ScratchVec::new();
        for _ in 0..100 {
            v.push(rc.clone());
        }
        let popped = v.pop();
        assert_eq!(Rc::strong_count(&rc), 101);
        drop(v);
        assert_eq!(Rc::strong_count(&rc), 2);
        drop(popped);
        assert_eq!(Rc::strong_count(&rc), 1);
    }

    #[test]
    fn string_writes_match_std() {
        use std::fmt::Write;
        let mut ours = ScratchString::default();
        let mut std = String::new();
        for i in 0..300 {
            write!(ours, "{i}-é,").unwrap();
            write!(std, "{i}-é,").unwrap();
            ours.push('ß');
            std.push('ß');
        }
        assert_eq!(ours.as_str(), std);
        assert_eq!(format!("{ours}"), std);
        assert_eq!(format!("{ours:?}"), format!("{std:?}"));
    }

    /// The three users of the arena (chains, overflow's list items, types' path segments) on
    /// one input, formatted as rustfmt formats it; small enough for Miri
    /// (`cargo +nightly miri test -p chloro-core --lib scratch`).
    #[test]
    fn formatting_through_the_arena_users() {
        let src = "fn f() {\n    let result = some_receiver.method_one::<Vec<u8>>(first_argument, \
                   second_argument).method_two(std::collections::HashMap::<String, usize>::new(), \
                   InFile::new(file_id, decl))?.method_three();\n    acc.push(Foo { a: 1 }.into());\n}\n";
        let expected = "fn f() {
    let result = some_receiver
        .method_one::<Vec<u8>>(first_argument, second_argument)
        .method_two(
            std::collections::HashMap::<String, usize>::new(),
            InFile::new(file_id, decl),
        )?
        .method_three();
    acc.push(Foo { a: 1 }.into());
}
";
        assert_eq!(arena_state().3, 0, "a fresh thread has no chunk");
        assert_eq!(crate::format_source(src), expected);
        let (_, _, live, chunks) = arena_state();
        assert_eq!(
            (live, chunks),
            (0, 1),
            "the arena was used, and is empty with its chunk kept"
        );
    }

    #[test]
    fn each_thread_has_its_own_arena() {
        let mut here = ScratchVec::with_capacity(8);
        here.push(1u64);
        let there = std::thread::spawn(|| {
            let mut v = ScratchVec::with_capacity(8);
            v.extend(0..1000u64);
            (v.iter().sum::<u64>(), arena_state().2)
        })
        .join()
        .unwrap();
        assert_eq!(there, (499_500, 1));
        assert_eq!(&here[..], &[1]);
    }
}
