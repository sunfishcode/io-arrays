//! Windows implementations of `ReadAt` and `WriteAt` functions.
//!
//! These can use `seek_read`/`seek_write` because the file's current position
//! is not exposed.

use std::{
    convert::TryInto,
    fs,
    io::{self, IoSlice, IoSliceMut},
    os::windows::fs::FileExt,
    slice,
};
use system_interface::fs::FileIoExt;

#[inline]
pub(crate) fn read_at(file: &fs::File, buf: &mut [u8], offset: u64) -> io::Result<usize> {
    file.seek_read(buf, offset)
}

pub(crate) fn read_exact_at(
    file: &fs::File,
    mut buf: &mut [u8],
    mut offset: u64,
) -> io::Result<()> {
    loop {
        match read_at(file, buf, offset) {
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

pub(crate) fn read_vectored_at(
    file: &fs::File,
    bufs: &mut [IoSliceMut],
    offset: u64,
) -> io::Result<usize> {
    let buf = bufs
        .iter_mut()
        .find(|b| !b.is_empty())
        .map_or(&mut [][..], |b| &mut **b);
    read_at(file, buf, offset)
}

pub(crate) fn read_exact_vectored_at(
    file: &fs::File,
    mut bufs: &mut [IoSliceMut],
    mut offset: u64,
) -> io::Result<()> {
    while !bufs.is_empty() {
        match read_vectored_at(file, bufs, offset) {
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

pub(crate) fn is_read_vectored_at(_file: &fs::File) -> bool {
    false
}

#[inline]
pub(crate) fn write_at(file: &fs::File, buf: &[u8], offset: u64) -> io::Result<usize> {
    file.seek_write(buf, offset)
}

pub(crate) fn write_all_at(file: &fs::File, mut buf: &[u8], mut offset: u64) -> io::Result<()> {
    loop {
        match write_at(file, buf, offset) {
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

pub(crate) fn write_vectored_at(
    file: &fs::File,
    bufs: &[IoSlice],
    offset: u64,
) -> io::Result<usize> {
    let buf = bufs
        .iter()
        .find(|b| !b.is_empty())
        .map_or(&[][..], |b| &**b);
    write_at(file, buf, offset)
}

pub(crate) fn write_all_vectored_at(
    file: &fs::File,
    mut bufs: &mut [IoSlice],
    mut offset: u64,
) -> io::Result<()> {
    while !bufs.is_empty() {
        match write_vectored_at(file, bufs, offset) {
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

#[inline]
pub(crate) fn allocate(file: &fs::File, offset: u64, len: u64) -> io::Result<()> {
    // FIXME: do a seek_write at the end to lengthen it? But that might
    // be racy. Maybe we should just have a set_len and pass the problem
    // on to our users.
    file.allocate(offset, len)
}

#[inline]
pub(crate) fn is_write_vectored_at(_file: &fs::File) -> bool {
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
