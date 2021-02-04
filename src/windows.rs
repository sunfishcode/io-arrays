//! Windows implementations of [`ReadAt`] and [`WriteAt`] functions for
//! file-like types which implement [`AsUnsafeFile`] on Windows.
//!
//! These can use `seek_read`/`seek_write` because the file's current position
//! is not exposed.
//!
//! [`ReadAt`]: crate::ReadAt
//! [`WriteAt`]: crate::WriteAt

use crate::Metadata;
use std::{
    convert::TryInto,
    io::{self, IoSlice, IoSliceMut},
    os::windows::fs::FileExt,
    slice,
};
use unsafe_io::AsUnsafeFile;
#[cfg(feature = "io-streams")]
use {
    crate::own_streamer::OwnStreamer,
    cap_fs_ext::{OpenOptions, Reopen},
    io_streams::StreamReader,
    std::io::SeekFrom,
    system_interface::fs::FileIoExt,
};

/// Implement [`crate::Range::metadata`].
#[inline]
pub fn metadata<Filelike: AsUnsafeFile>(filelike: &Filelike) -> io::Result<Metadata> {
    filelike.as_file_view().metadata().map(|meta| {
        Metadata {
            len: meta.len(),

            // Windows doesn't have a convenient way to query this, but
            // it often uses this specific value.
            blksize: 0x1000,
        }
    })
}

/// Implement [`crate::ReadAt::read_at`].
#[inline]
pub fn read_at<Filelike: AsUnsafeFile>(
    filelike: &Filelike,
    buf: &mut [u8],
    offset: u64,
) -> io::Result<usize> {
    filelike.as_file_view().seek_read(buf, offset)
}

/// Implement [`crate::ReadAt::read_exact_at`].
pub fn read_exact_at<Filelike: AsUnsafeFile>(
    filelike: &Filelike,
    mut buf: &mut [u8],
    mut offset: u64,
) -> io::Result<()> {
    loop {
        match read_at(filelike, buf, offset) {
            Ok(0) if !buf.is_empty() => {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "failed to fill whole buffer",
                ))
            }
            Ok(nread) => {
                offset = offset
                    .checked_add(nread.try_into().unwrap())
                    .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "offset overflow"))?;
                buf = &mut buf[nread..];
                if buf.is_empty() {
                    return Ok(());
                }
            }
            Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
            Err(err) => return Err(err),
        }
    }
}

/// Implement [`crate::ReadAt::read_vectored_at`].
pub fn read_vectored_at<Filelike: AsUnsafeFile>(
    filelike: &Filelike,
    bufs: &mut [IoSliceMut],
    offset: u64,
) -> io::Result<usize> {
    let buf = bufs
        .iter_mut()
        .find(|b| !b.is_empty())
        .map_or(&mut [][..], |b| &mut **b);
    read_at(filelike, buf, offset)
}

/// Implement [`crate::ReadAt::read_exact_vectored_at`].
pub fn read_exact_vectored_at<Filelike: AsUnsafeFile>(
    filelike: &Filelike,
    mut bufs: &mut [IoSliceMut],
    mut offset: u64,
) -> io::Result<()> {
    while !bufs.is_empty() {
        match read_vectored_at(filelike, bufs, offset) {
            Ok(nread) => {
                offset = offset
                    .checked_add(nread.try_into().unwrap())
                    .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "offset overflow"))?;
                bufs = advance_mut(bufs, nread);
            }
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => (),
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

/// Implement [`crate::ReadAt::is_read_vectored_at`].
#[inline]
pub fn is_read_vectored_at<Filelike: AsUnsafeFile>(_filelike: &Filelike) -> bool {
    false
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
    filelike.as_file_view().seek_write(buf, offset)
}

/// Implement [`crate::WriteAt::write_all_at`].
pub fn write_all_at<Filelike: AsUnsafeFile>(
    filelike: &Filelike,
    mut buf: &[u8],
    mut offset: u64,
) -> io::Result<()> {
    loop {
        match write_at(filelike, buf, offset) {
            Ok(nwritten) => {
                offset = offset
                    .checked_add(nwritten.try_into().unwrap())
                    .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "offset overflow"))?;
                buf = &buf[nwritten..];
                if buf.is_empty() {
                    return Ok(());
                }
            }
            Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
            Err(err) => return Err(err),
        }
    }
}

/// Implement [`crate::WriteAt::write_vectored_at`].
pub fn write_vectored_at<Filelike: AsUnsafeFile>(
    filelike: &Filelike,
    bufs: &[IoSlice],
    offset: u64,
) -> io::Result<usize> {
    let buf = bufs
        .iter()
        .find(|b| !b.is_empty())
        .map_or(&[][..], |b| &**b);
    write_at(filelike, buf, offset)
}

/// Implement [`crate::WriteAt::write_all_vectored_at`].
pub fn write_all_vectored_at<Filelike: AsUnsafeFile>(
    filelike: &Filelike,
    mut bufs: &mut [IoSlice],
    mut offset: u64,
) -> io::Result<()> {
    while !bufs.is_empty() {
        match write_vectored_at(filelike, bufs, offset) {
            Ok(nwritten) => {
                offset = offset
                    .checked_add(nwritten.try_into().unwrap())
                    .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "offset overflow"))?;
                bufs = advance(bufs, nwritten);
            }
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => (),
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

/// Implement [`crate::WriteAt::is_write_vectored_at`].
#[inline]
pub fn is_write_vectored_at<Filelike: AsUnsafeFile>(_filelike: &Filelike) -> bool {
    false
}

/// This will be obviated by [rust-lang/rust#62726].
///
/// [rust-lang/rust#62726]: https://github.com/rust-lang/rust/issues/62726.
fn advance<'a, 'b>(bufs: &'b mut [IoSlice<'a>], n: usize) -> &'b mut [IoSlice<'a>] {
    // Number of buffers to remove.
    let mut remove = 0;
    // Total length of all the to be removed buffers.
    let mut accumulated_len = 0;
    for buf in bufs.iter() {
        if accumulated_len + buf.len() > n {
            break;
        } else {
            accumulated_len += buf.len();
            remove += 1;
        }
    }

    #[allow(clippy::indexing_slicing)]
    let bufs = &mut bufs[remove..];
    if let Some(first) = bufs.first_mut() {
        let advance_by = n - accumulated_len;
        let mut ptr = first.as_ptr();
        let mut len = first.len();
        unsafe {
            ptr = ptr.add(advance_by);
            len -= advance_by;
            *first = IoSlice::<'a>::new(slice::from_raw_parts::<'a>(ptr, len));
        }
    }
    bufs
}

/// This will be obviated by [rust-lang/rust#62726].
///
/// [rust-lang/rust#62726]: https://github.com/rust-lang/rust/issues/62726.
fn advance_mut<'a, 'b>(bufs: &'b mut [IoSliceMut<'a>], n: usize) -> &'b mut [IoSliceMut<'a>] {
    // Number of buffers to remove.
    let mut remove = 0;
    // Total length of all the to be removed buffers.
    let mut accumulated_len = 0;
    for buf in bufs.iter() {
        if accumulated_len + buf.len() > n {
            break;
        } else {
            accumulated_len += buf.len();
            remove += 1;
        }
    }

    #[allow(clippy::indexing_slicing)]
    let bufs = &mut bufs[remove..];
    if let Some(first) = bufs.first_mut() {
        let advance_by = n - accumulated_len;
        let mut ptr = first.as_mut_ptr();
        let mut len = first.len();
        unsafe {
            ptr = ptr.add(advance_by);
            len -= advance_by;
            *first = IoSliceMut::<'a>::new(slice::from_raw_parts_mut::<'a>(ptr, len));
        }
    }
    bufs
}
