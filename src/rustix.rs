//! Functions for implementing [`ReadAt`] and [`WriteAt`] for file-like types
//! which implement [`AsFilelike`] on Posix-ish platforms.
//!
//! [`ReadAt`]: crate::ReadAt
//! [`WriteAt`]: crate::WriteAt

use crate::Metadata;
use io_lifetimes::AsFilelike;
use std::fs::File;
use std::io::{self, IoSlice, IoSliceMut};
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
#[cfg(target_os = "wasi")]
use std::os::unix::fs::MetadataExt;
use system_interface::fs::FileIoExt;
#[cfg(feature = "io-streams")]
use {
    crate::owned_streamer::OwnedStreamer,
    cap_fs_ext::{OpenOptions, Reopen},
    io_streams::StreamReader,
    std::io::SeekFrom,
};

/// Implement [`crate::Array::metadata`].
#[inline]
pub fn metadata<'a, Filelike: AsFilelike>(filelike: &Filelike) -> io::Result<Metadata> {
    filelike
        .as_filelike_view::<File>()
        .metadata()
        .map(|meta| Metadata {
            len: meta.len(),
            blksize: meta.blksize(),
        })
}

/// Implement [`crate::ReadAt::read_at`].
#[inline]
pub fn read_at<'a, Filelike: AsFilelike>(
    filelike: &Filelike,
    buf: &mut [u8],
    offset: u64,
) -> io::Result<usize> {
    <File as FileIoExt>::read_at(&filelike.as_filelike_view::<File>(), buf, offset)
}

/// Implement [`crate::ReadAt::read_exact_at`].
#[inline]
pub fn read_exact_at<'a, Filelike: AsFilelike>(
    filelike: &Filelike,
    buf: &mut [u8],
    offset: u64,
) -> io::Result<()> {
    <File as FileIoExt>::read_exact_at(&filelike.as_filelike_view::<File>(), buf, offset)
}

/// Implement [`crate::ReadAt::read_vectored_at`].
#[inline]
pub fn read_vectored_at<'a, Filelike: AsFilelike>(
    filelike: &Filelike,
    bufs: &mut [IoSliceMut],
    offset: u64,
) -> io::Result<usize> {
    <File as FileIoExt>::read_vectored_at(&filelike.as_filelike_view::<File>(), bufs, offset)
}

/// Implement [`crate::ReadAt::read_exact_vectored_at`].
#[inline]
pub fn read_exact_vectored_at<'a, Filelike: AsFilelike>(
    filelike: &Filelike,
    bufs: &mut [IoSliceMut],
    offset: u64,
) -> io::Result<()> {
    <File as FileIoExt>::read_exact_vectored_at(&filelike.as_filelike_view::<File>(), bufs, offset)
}

/// Implement [`crate::ReadAt::is_read_vectored_at`].
#[inline]
pub fn is_read_vectored_at<'a, Filelike: AsFilelike>(filelike: &Filelike) -> bool {
    <File as FileIoExt>::is_read_vectored_at(&filelike.as_filelike_view::<File>())
}

/// Implement [`crate::ReadAt::read_via_stream_at`].
#[cfg(feature = "io-streams")]
pub fn read_via_stream_at<'a, Filelike: AsFilelike>(
    filelike: &Filelike,
    offset: u64,
) -> io::Result<StreamReader> {
    // On operating systems where we can do so, reopen the file so that we
    // get an independent current position.
    let view = filelike.as_filelike_view::<File>();
    if let Ok(file) = view.reopen(OpenOptions::new().read(true)) {
        if offset != 0 {
            file.seek(SeekFrom::Start(offset))?;
        }
        return Ok(StreamReader::file(file));
    }

    // Otherwise, manually stream the file.
    StreamReader::piped_thread(Box::new(OwnedStreamer::new(view.try_clone()?, offset)))
}

/// Implement [`crate::WriteAt::write_at`].
#[inline]
pub fn write_at<'a, Filelike: AsFilelike>(
    filelike: &Filelike,
    buf: &[u8],
    offset: u64,
) -> io::Result<usize> {
    <File as FileIoExt>::write_at(&filelike.as_filelike_view::<File>(), buf, offset)
}

/// Implement [`crate::WriteAt::write_all_at`].
#[inline]
pub fn write_all_at<'a, Filelike: AsFilelike>(
    filelike: &Filelike,
    buf: &[u8],
    offset: u64,
) -> io::Result<()> {
    <File as FileIoExt>::write_all_at(&filelike.as_filelike_view::<File>(), buf, offset)
}

/// Implement [`crate::WriteAt::write_vectored_at`].
#[inline]
pub fn write_vectored_at<'a, Filelike: AsFilelike>(
    filelike: &Filelike,
    bufs: &[IoSlice],
    offset: u64,
) -> io::Result<usize> {
    <File as FileIoExt>::write_vectored_at(&filelike.as_filelike_view::<File>(), bufs, offset)
}

/// Implement [`crate::WriteAt::write_all_vectored_at`].
#[inline]
pub fn write_all_vectored_at<'a, Filelike: AsFilelike>(
    filelike: &Filelike,
    bufs: &mut [IoSlice],
    offset: u64,
) -> io::Result<()> {
    <File as FileIoExt>::write_all_vectored_at(&filelike.as_filelike_view::<File>(), bufs, offset)
}

/// Implement [`crate::WriteAt::is_write_vectored_at`].
#[inline]
pub fn is_write_vectored_at<'a, Filelike: AsFilelike>(filelike: &Filelike) -> bool {
    <File as FileIoExt>::is_write_vectored_at(&filelike.as_filelike_view::<File>())
}
