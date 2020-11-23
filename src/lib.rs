//! Random-access I/O
//!
//! For a starting point, see [`FileReader`] and [`FileWriter`] for input and
//! output. There's also [`FileEditor`] for combination input and output.

#![deny(missing_docs)]

#[cfg(feature = "io-streams")]
mod file_streamer;
mod files;

pub use files::{
    EditAt, FileEditor, FileReader, FileWriter, Metadata, MinimalFile, ReadAt, WriteAt,
};
