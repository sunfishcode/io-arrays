//! Functions for implementing [`ReadAt`] and [`WriteAt`] for file-like types
//! which implement [`AsUnsafeFile`] on Posix-ish platforms.
//!
//! [`WriteAt`]: crate::WriteAt

use crate::{
    borrow_streamer::{BorrowStreamer, BorrowStreamerMut},
    Advice, Metadata, ReadAt,
};
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
#[cfg(target_os = "wasi")]
use std::os::unix::fs::MetadataExt;
use std::{
    fs,
    io::{self, copy, IoSlice, IoSliceMut, Read},
};
use system_interface::fs::FileIoExt;
use unsafe_io::AsUnsafeFile;
#[cfg(feature = "io-streams")]
use {
    crate::own_streamer::OwnStreamer,
    cap_fs_ext::{OpenOptions, Reopen},
    io_streams::StreamReader,
    std::io::SeekFrom,
};

/// Implement [`crate::Range::metadata`].
#[inline]
pub fn metadata<Filelike: AsUnsafeFile>(filelike: &Filelike) -> io::Result<Metadata> {
    filelike.as_file_view().metadata().map(|meta| Metadata {
        len: meta.len(),
        blksize: meta.blksize(),
    })
}

/// Implement [`crate::ReadAt::read_at`].
#[inline]
pub fn read_at<Filelike: AsUnsafeFile>(
    filelike: &Filelike,
    buf: &mut [u8],
    offset: u64,
) -> io::Result<usize> {
    <fs::File as FileIoExt>::read_at(&filelike.as_file_view(), buf, offset)
}

/// Implement [`crate::ReadAt::read_exact_at`].
#[inline]
pub fn read_exact_at<Filelike: AsUnsafeFile>(
    filelike: &Filelike,
    buf: &mut [u8],
    offset: u64,
) -> io::Result<()> {
    <fs::File as FileIoExt>::read_exact_at(&filelike.as_file_view(), buf, offset)
}

/// Implement [`crate::ReadAt::read_vectored_at`].
#[inline]
pub fn read_vectored_at<Filelike: AsUnsafeFile>(
    filelike: &Filelike,
    bufs: &mut [IoSliceMut],
    offset: u64,
) -> io::Result<usize> {
    <fs::File as FileIoExt>::read_vectored_at(&filelike.as_file_view(), bufs, offset)
}

/// Implement [`crate::ReadAt::read_exact_vectored_at`].
#[inline]
pub fn read_exact_vectored_at<Filelike: AsUnsafeFile>(
    filelike: &Filelike,
    bufs: &mut [IoSliceMut],
    offset: u64,
) -> io::Result<()> {
    <fs::File as FileIoExt>::read_exact_vectored_at(&filelike.as_file_view(), bufs, offset)
}

/// Implement [`crate::ReadAt::is_read_vectored_at`].
#[inline]
pub fn is_read_vectored_at<Filelike: AsUnsafeFile>(filelike: &Filelike) -> bool {
    <fs::File as FileIoExt>::is_read_vectored_at(&filelike.as_file_view())
}

/// Implement [`crate::ReadAt::read_via_stream_at`].
#[cfg(feature = "io-streams")]
pub fn read_via_stream_at<Filelike: AsUnsafeFile>(
    filelike: &Filelike,
    offset: u64,
) -> io::Result<StreamReader> {
    // On operating systems where we can do so, reopen the file so that we
    // get an independent current position.
    if let Ok(file) = filelike
        .as_file_view()
        .reopen(OpenOptions::new().read(true))
    {
        if offset != 0 {
            file.seek(SeekFrom::Start(offset))?;
        }
        return Ok(StreamReader::file(file));
    }

    // Otherwise, manually stream the file.
    StreamReader::piped_thread(Box::new(OwnStreamer::new(
        filelike.as_file_view().try_clone()?,
        offset,
    )))
}

/// Implement [`crate::WriteAt::write_at`].
#[inline]
pub fn write_at<Filelike: AsUnsafeFile>(
    filelike: &Filelike,
    buf: &[u8],
    offset: u64,
) -> io::Result<usize> {
    <fs::File as FileIoExt>::write_at(&filelike.as_file_view(), buf, offset)
}

/// Implement [`crate::WriteAt::write_all_at`].
#[inline]
pub fn write_all_at<Filelike: AsUnsafeFile>(
    filelike: &Filelike,
    buf: &[u8],
    offset: u64,
) -> io::Result<()> {
    <fs::File as FileIoExt>::write_all_at(&filelike.as_file_view(), buf, offset)
}

/// Implement [`crate::WriteAt::write_vectored_at`].
#[inline]
pub fn write_vectored_at<Filelike: AsUnsafeFile>(
    filelike: &Filelike,
    bufs: &[IoSlice],
    offset: u64,
) -> io::Result<usize> {
    <fs::File as FileIoExt>::write_vectored_at(&filelike.as_file_view(), bufs, offset)
}

/// Implement [`crate::WriteAt::write_all_vectored_at`].
#[inline]
pub fn write_all_vectored_at<Filelike: AsUnsafeFile>(
    filelike: &Filelike,
    bufs: &mut [IoSlice],
    offset: u64,
) -> io::Result<()> {
    <fs::File as FileIoExt>::write_all_vectored_at(&filelike.as_file_view(), bufs, offset)
}

/// Implement [`crate::WriteAt::is_write_vectored_at`].
#[inline]
pub fn is_write_vectored_at<Filelike: AsUnsafeFile>(filelike: &Filelike) -> bool {
    <fs::File as FileIoExt>::is_write_vectored_at(&filelike.as_file_view())
}
