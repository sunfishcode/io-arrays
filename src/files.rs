//! Common code for Posixh-ish and Windows platforms for implementing
//! [`ReadAt`] and [`WriteAt`] for file-like types which implement
//! [`AsUnsafeFile`].
//!
//! [`WriteAt`]: crate::WriteAt

use crate::{
    borrow_streamer::{BorrowStreamer, BorrowStreamerMut},
    Advice, ReadAt,
};
use std::{
    fs,
    io::{self, copy, Read},
};
use system_interface::fs::FileIoExt;
use unsafe_io::AsUnsafeFile;

/// Implement [`crate::Range::advise`].
#[inline]
pub fn advise<Filelike: AsUnsafeFile>(
    filelike: &Filelike,
    offset: u64,
    len: u64,
    advice: Advice,
) -> io::Result<()> {
    <fs::File as FileIoExt>::advise(&filelike.as_file_view(), offset, len, advice)
}

/// Implement [`crate::WriteAt::copy_from`].
#[inline]
pub fn copy_from<Filelike: AsUnsafeFile, R: ReadAt>(
    filelike: &mut Filelike,
    offset: u64,
    input: &R,
    input_offset: u64,
    len: u64,
) -> io::Result<u64> {
    let mut input_view = filelike.as_file_view();
    let mut output_streamer = BorrowStreamerMut::new(&mut *input_view, offset);
    let input_streamer = BorrowStreamer::new(input, input_offset);
    copy(&mut input_streamer.take(len), &mut output_streamer)
}

/// Implement [`crate::WriteAt::set_len`].
#[inline]
pub fn set_len<Filelike: AsUnsafeFile>(filelike: &mut Filelike, size: u64) -> io::Result<()> {
    filelike.as_file_view().set_len(size)
}
