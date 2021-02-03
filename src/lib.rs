//! Random-access I/O
//!
//! For a starting point, see [`RangeReader`] and [`RangeWriter`] for input and
//! output. There's also [`RangeEditor`] for combination input and output.

#![deny(missing_docs)]
#![cfg_attr(can_vector, feature(can_vector))]
#![cfg_attr(write_all_vectored, feature(write_all_vectored))]

mod borrow_streamer;
mod ranges;
#[cfg(feature = "io-streams")]
mod own_streamer;
#[cfg(not(windows))]
mod posish;
mod slice;
#[cfg(windows)]
mod windows;

/// Functions for implementing `ReadAt` and `WriteAt` for file-like types.
pub mod filelike {
    // We can't use Windows' `read_at` or `write_at` here because it isn't able to
    // extend the length of a file we can't `reopen` (such as temporary files).
    // However, while `FileIoExt` can't use `seek_write` because it mutates the
    // current position, here we *can* use plain `seek_write` because `RangeEditor`
    // doesn't expose the current position.
    #[cfg(not(windows))]
    pub use crate::posish::*;
    #[cfg(windows)]
    pub use crate::windows::*;
}

pub use ranges::{EditAt, Metadata, Range, RangeEditor, RangeReader, RangeWriter, ReadAt, WriteAt};

pub use system_interface::fs::Advice;
