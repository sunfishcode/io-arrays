//! Random-access I/O
//!
//! For a starting point, see [`FileReader`] and [`FileWriter`] for input and
//! output. There's also [`FileEditor`] for combination input and output.

#![deny(missing_docs)]
#![cfg_attr(can_vector, feature(can_vector))]
#![cfg_attr(write_all_vectored, feature(write_all_vectored))]

mod borrow_streamer;
#[cfg(feature = "io-streams")]
mod file_streamer;
mod files;
#[cfg(windows)]
mod windows;

pub use files::{
    EditAt, FileEditor, FileReader, FileWriter, Metadata, MinimalFile, ReadAt, WriteAt,
};
