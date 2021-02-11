//! Random-access I/O
//!
//! This crate defines [`ReadAt`], [`WriteAt`], and [`EditAt`] traits which
//! define interfaces to random-access or seekable devices, such as normal
//! files, block devices, disk partitions, and memory buffers.
//!
//! It also defines [`ArrayReader`], [`ArrayWriter`], and [`ArrayEditor`] types
//! which implement the above traits and and can be constructed from any
//! file-like type.  On Posix-ish platforms, including limited support for
//! WASI, these types just contain a single file descriptor (and implement
//! [`AsRawFd`]), plus any resources needed to safely hold the file descriptor
//! live. On Windows, they contain a single file handle (and implement
//! [`AsRawHandle`]).
//!
//! [`AsRawFd`]: https://doc.rust-lang.org/std/os/unix/io/trait.AsRawFd.html
//! [`AsRawHandle`]: https://doc.rust-lang.org/std/os/windows/io/trait.AsRawHandle.html

#![deny(missing_docs)]
#![cfg_attr(can_vector, feature(can_vector))]
#![cfg_attr(write_all_vectored, feature(write_all_vectored))]

mod arrays;
mod borrow_streamer;
mod files;
#[cfg(feature = "io-streams")]
mod owned_streamer;
#[cfg(not(windows))]
mod posish;
mod slice;
#[cfg(windows)]
mod windows;

pub use arrays::{Array, ArrayEditor, ArrayReader, ArrayWriter, EditAt, Metadata, ReadAt, WriteAt};

/// Advice to pass to [`Array::advise`] to describe an expected access pattern.
///
/// This is a re-export of [`system_interface::fs::Advice`].
pub use system_interface::fs::Advice;

/// Functions for custom implementations of [`ReadAt`] and [`WriteAt`] for
/// file-like types.
pub mod filelike {
    // We can't use Windows' `read_at` or `write_at` here because it isn't able to
    // extend the length of a file we can't `reopen` (such as temporary files).
    // However, while `FileIoExt` can't use `seek_write` because it mutates the
    // current position, here we *can* use plain `seek_write` because `ArrayEditor`
    // doesn't expose the current position.
    pub use crate::files::{advise, copy_from, set_len};
    #[cfg(all(not(windows), feature = "io-streams"))]
    pub use crate::posish::read_via_stream_at;
    #[cfg(not(windows))]
    pub use crate::posish::{
        is_read_vectored_at, is_write_vectored_at, metadata, read_at, read_exact_at,
        read_exact_vectored_at, read_vectored_at, write_all_at, write_all_vectored_at, write_at,
        write_vectored_at,
    };
    #[cfg(all(windows, feature = "io-streams"))]
    pub use crate::windows::read_via_stream_at;
    #[cfg(windows)]
    pub use crate::windows::{
        is_read_vectored_at, is_write_vectored_at, metadata, read_at, read_exact_at,
        read_exact_vectored_at, read_vectored_at, write_all_at, write_all_vectored_at, write_at,
        write_vectored_at,
    };
}
