//! Common code for Posix-ish and Windows platforms for implementing
//! [`ReadAt`] and [`WriteAt`] for file-like types which implement
//! [`AsFilelike`].
//!
//! [`WriteAt`]: crate::WriteAt

use crate::{
    borrow_streamer::{BorrowStreamer, BorrowStreamerMut},
    Advice, ReadAt,
};
use io_lifetimes::AsFilelike;
use std::{
    fs::File,
    io::{self, copy, Read},
};
use system_interface::fs::FileIoExt;

/// Implement [`crate::Array::advise`].
#[inline]
pub fn advise<'f, Filelike: AsFilelike>(
    filelike: &Filelike,
    offset: u64,
    len: u64,
    advice: Advice,
) -> io::Result<()> {
    <File as FileIoExt>::advise(&filelike.as_filelike_view::<File>(), offset, len, advice)
}

/// Implement [`crate::WriteAt::copy_from`].
#[inline]
pub fn copy_from<'f, Filelike: AsFilelike, R: ReadAt>(
    filelike: &Filelike,
    offset: u64,
    input: &R,
    input_offset: u64,
    len: u64,
) -> io::Result<u64> {
    let mut input_view = filelike.as_filelike_view::<File>();
    let mut output_streamer = BorrowStreamerMut::new(&mut *input_view, offset);
    let input_streamer = BorrowStreamer::new(input, input_offset);
    copy(&mut input_streamer.take(len), &mut output_streamer)
}

/// Implement [`crate::WriteAt::set_len`].
#[inline]
pub fn set_len<'f, Filelike: AsFilelike>(filelike: &Filelike, size: u64) -> io::Result<()> {
    filelike.as_filelike_view::<File>().set_len(size)
}
