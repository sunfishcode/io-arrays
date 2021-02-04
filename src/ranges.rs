use crate::{filelike, Advice};
#[cfg(feature = "io-streams")]
use io_streams::StreamReader;
#[cfg(unix)]
use std::os::unix::io::{AsRawFd, RawFd};
#[cfg(target_os = "wasi")]
use std::os::wasi::io::{AsRawFd, RawFd};
#[cfg(windows)]
use std::os::windows::io::{AsRawHandle, RawHandle};
use std::{
    fs,
    io::{self, IoSlice, IoSliceMut, Read, Seek, Write},
};
use system_interface::fs::FileIoExt;
use unsafe_io::{AsUnsafeFile, FromUnsafeFile, IntoUnsafeFile, UnsafeFile};

/// Metadata information about a range.
pub struct Metadata {
    pub(crate) len: u64,
    pub(crate) blksize: u64,
}

#[allow(clippy::len_without_is_empty)]
impl Metadata {
    /// Returns the size of the range, in bytes, this metadata is for.
    #[inline]
    #[must_use]
    pub const fn len(&self) -> u64 {
        self.len
    }

    /// Returns the block size for I/O.
    #[inline]
    #[must_use]
    pub const fn blksize(&self) -> u64 {
        self.blksize
    }
}

/// A minimal base trait for range I/O. Defines operations common to all kinds
/// of random-access devices that fit the "range" concept, including normal
/// files, block devices, and in-memory buffers.
pub trait Range {
    /// Return the `Metadata` for the range. This is similar to
    /// `std::fs::File::metadata`, though it returns fewer fields since the
    /// underlying device may not be an actual filesystem inode.
    fn metadata(&self) -> io::Result<Metadata>;

    /// Announce the expected access pattern of the data at the given offset.
    fn advise(&self, offset: u64, len: u64, advice: Advice) -> io::Result<()>;
}

/// A trait for reading from ranges.
///
/// Unlike `std::io::Read`, `ReadAt`'s functions take a `&self` rather than a
/// `&mut self`, since they don't have a current position to mutate.
pub trait ReadAt: Range {
    /// Reads a number of bytes starting from a given offset.
    ///
    /// This is similar to [`std::os::unix::fs::FileExt::read_at`], except it
    /// takes `self` by immutable reference since the entire side effect is
    /// I/O, and it's supported on non-Unix platforms including Windows.
    ///
    /// [`std::os::unix::fs::FileExt::read_at`]: https://doc.rust-lang.org/std/os/unix/fs/trait.FileExt.html#tymethod.read_at
    fn read_at(&self, buf: &mut [u8], offset: u64) -> io::Result<usize>;

    /// Reads the exact number of byte required to fill buf from the given
    /// offset.
    ///
    /// This is similar to [`std::os::unix::fs::FileExt::read_exact_at`], except
    /// it takes `self` by immutable reference since the entire side effect is
    /// I/O, and it's supported on non-Unix platforms including Windows.
    ///
    /// [`std::os::unix::fs::FileExt::read_exact_at`]: https://doc.rust-lang.org/std/os/unix/fs/trait.FileExt.html#tymethod.read_exact_at
    fn read_exact_at(&self, buf: &mut [u8], offset: u64) -> io::Result<()>;

    /// Is to `read_vectored` what `read_at` is to `read`.
    fn read_vectored_at(&self, bufs: &mut [IoSliceMut], offset: u64) -> io::Result<usize>;

    /// Is to `read_exact_vectored` what `read_exact_at` is to `read_exact`.
    fn read_exact_vectored_at(&self, bufs: &mut [IoSliceMut], offset: u64) -> io::Result<()>;

    /// Determines if `Self` has an efficient `read_vectored_at` implementation.
    fn is_read_vectored_at(&self) -> bool;

    /// Create a `StreamReader` which reads from the range at the given offset.
    #[cfg(feature = "io-streams")]
    fn read_via_stream_at(&self, offset: u64) -> io::Result<StreamReader>;
}

/// A trait for writing to ranges.
pub trait WriteAt: Range {
    /// Writes a number of bytes starting from a given offset.
    ///
    /// This is similar to [`std::os::unix::fs::FileExt::write_at`], except it
    /// takes `self` by immutable reference since the entire side effect is
    /// I/O, and it's supported on non-Unix platforms including Windows.
    ///
    /// [`std::os::unix::fs::FileExt::write_at`]: https://doc.rust-lang.org/std/os/unix/fs/trait.FileExt.html#tymethod.write_at
    fn write_at(&mut self, buf: &[u8], offset: u64) -> io::Result<usize>;

    /// Attempts to write an entire buffer starting from a given offset.
    ///
    /// This is similar to [`std::os::unix::fs::FileExt::write_all_at`], except
    /// it takes `self` by immutable reference since the entire side effect is
    /// I/O, and it's supported on non-Unix platforms including Windows.
    ///
    /// [`std::os::unix::fs::FileExt::write_all_at`]: https://doc.rust-lang.org/std/os/unix/fs/trait.FileExt.html#tymethod.write_all_at
    fn write_all_at(&mut self, buf: &[u8], offset: u64) -> io::Result<()>;

    /// Is to `write_vectored` what `write_at` is to `write`.
    fn write_vectored_at(&mut self, bufs: &[IoSlice], offset: u64) -> io::Result<usize>;

    /// Is to `write_all_vectored` what `write_all_at` is to `write_all`.
    fn write_all_vectored_at(&mut self, bufs: &mut [IoSlice], offset: u64) -> io::Result<()>;

    /// Determines if `Self` has an efficient `write_vectored_at` implementation.
    fn is_write_vectored_at(&self) -> bool;

    /// Copy `len` bytes from `input` at `input_offset` to `self` at `offset`.
    fn copy_from<R: ReadAt>(
        &mut self,
        offset: u64,
        input: &R,
        input_offset: u64,
        len: u64,
    ) -> io::Result<u64>;

    /// Truncates or extends the underlying range, updating the size of this
    /// range to become `size`.
    fn set_len(&mut self, size: u64) -> io::Result<()>;
}

/// A trait for reading and writing to ranges.
pub trait EditAt: ReadAt + WriteAt {}

/// A random-access input source.
#[derive(Debug)]
pub struct RangeReader {
    file: fs::File,
}

/// A random-access output source.
#[derive(Debug)]
pub struct RangeWriter {
    file: fs::File,
}

/// A random-access input and output source.
#[derive(Debug)]
pub struct RangeEditor {
    file: fs::File,
}

impl RangeReader {
    /// Convert a `File` into a `RangeReader`.
    #[inline]
    #[must_use]
    pub fn file<Filelike: IntoUnsafeFile + Read + Seek>(filelike: Filelike) -> Self {
        Self {
            file: fs::File::from_filelike(filelike),
        }
    }

    /// Copy a slice of bytes into a memory buffer to allow it to be accessed
    /// in the manner of a range.
    #[inline]
    pub fn bytes(bytes: &[u8]) -> io::Result<Self> {
        let unsafe_file = create_anonymous()?;
        unsafe_file.as_file_view().write_all(bytes)?;
        Ok(Self {
            file: unsafe { fs::File::from_unsafe_file(unsafe_file) },
        })
    }
}

impl RangeWriter {
    /// Convert a `File` into a `RangeWriter`.
    ///
    /// The file must not be opened in [append mode].
    ///
    /// [append mode]: https://doc.rust-lang.org/stable/std/fs/struct.OpenOptions.html#method.append
    #[inline]
    #[must_use]
    pub fn file<Filelike: IntoUnsafeFile + Write + Seek>(filelike: Filelike) -> Self {
        Self::_file(fs::File::from_filelike(filelike))
    }

    #[inline]
    fn _file(file: fs::File) -> Self {
        // On Linux, `pwrite` on a file opened with `O_APPEND` writes to the
        // end of the file, ignoring the offset.
        #[cfg(not(windows))]
        {
            assert!(
                !posish::fs::getfl(&file)
                    .unwrap()
                    .contains(posish::fs::OFlags::APPEND),
                "RangeWriter doesn't support files opened with O_APPEND"
            );
        }
        #[cfg(windows)]
        {
            assert!(
                (winx::file::query_access_information(file.as_raw_handle()).unwrap()
                    & winx::file::AccessMode::FILE_APPEND_DATA)
                    == winx::file::AccessMode::FILE_APPEND_DATA,
                "RangeWriter doesn't support files opened with FILE_APPEND_DATA"
            );
        }

        Self { file }
    }
}

impl RangeEditor {
    /// Convert a `File` into a `RangeEditor`.
    #[inline]
    #[must_use]
    pub fn file<Filelike: IntoUnsafeFile + Read + Write + Seek>(filelike: Filelike) -> Self {
        Self {
            file: fs::File::from_filelike(filelike),
        }
    }

    /// Create a temporary anonymous resource which can be accessed in the
    /// manner of a file.
    #[inline]
    pub fn anonymous() -> io::Result<Self> {
        let file = create_anonymous()?;
        Ok(Self {
            file: unsafe { fs::File::from_unsafe_file(file) },
        })
    }
}

impl Range for RangeReader {
    #[inline]
    fn metadata(&self) -> io::Result<Metadata> {
        filelike::metadata(self)
    }

    #[inline]
    fn advise(&self, offset: u64, len: u64, advice: Advice) -> io::Result<()> {
        filelike::advise(self, offset, len, advice)
    }
}

impl Range for RangeWriter {
    #[inline]
    fn metadata(&self) -> io::Result<Metadata> {
        filelike::metadata(self)
    }

    #[inline]
    fn advise(&self, offset: u64, len: u64, advice: Advice) -> io::Result<()> {
        filelike::advise(self, offset, len, advice)
    }
}

impl Range for RangeEditor {
    #[inline]
    fn metadata(&self) -> io::Result<Metadata> {
        filelike::metadata(self)
    }

    #[inline]
    fn advise(&self, offset: u64, len: u64, advice: Advice) -> io::Result<()> {
        filelike::advise(self, offset, len, advice)
    }
}

impl ReadAt for RangeReader {
    #[inline]
    fn read_at(&self, buf: &mut [u8], offset: u64) -> io::Result<usize> {
        filelike::read_at(self, buf, offset)
    }

    #[inline]
    fn read_exact_at(&self, buf: &mut [u8], offset: u64) -> io::Result<()> {
        filelike::read_exact_at(self, buf, offset)
    }

    #[inline]
    fn read_vectored_at(&self, bufs: &mut [IoSliceMut], offset: u64) -> io::Result<usize> {
        filelike::read_vectored_at(self, bufs, offset)
    }

    #[inline]
    fn read_exact_vectored_at(&self, bufs: &mut [IoSliceMut], offset: u64) -> io::Result<()> {
        filelike::read_exact_vectored_at(self, bufs, offset)
    }

    #[inline]
    fn is_read_vectored_at(&self) -> bool {
        filelike::is_read_vectored_at(self)
    }

    #[cfg(feature = "io-streams")]
    #[inline]
    fn read_via_stream_at(&self, offset: u64) -> io::Result<StreamReader> {
        filelike::read_via_stream_at(self, offset)
    }
}

impl ReadAt for RangeEditor {
    #[inline]
    fn read_at(&self, buf: &mut [u8], offset: u64) -> io::Result<usize> {
        filelike::read_at(self, buf, offset)
    }

    #[inline]
    fn read_exact_at(&self, buf: &mut [u8], offset: u64) -> io::Result<()> {
        filelike::read_exact_at(self, buf, offset)
    }

    #[inline]
    fn read_vectored_at(&self, bufs: &mut [IoSliceMut], offset: u64) -> io::Result<usize> {
        filelike::read_vectored_at(self, bufs, offset)
    }

    #[inline]
    fn read_exact_vectored_at(&self, bufs: &mut [IoSliceMut], offset: u64) -> io::Result<()> {
        filelike::read_exact_vectored_at(self, bufs, offset)
    }

    #[inline]
    fn is_read_vectored_at(&self) -> bool {
        filelike::is_read_vectored_at(self)
    }

    #[cfg(feature = "io-streams")]
    #[inline]
    fn read_via_stream_at(&self, offset: u64) -> io::Result<StreamReader> {
        filelike::read_via_stream_at(self, offset)
    }
}

impl WriteAt for RangeWriter {
    fn write_at(&mut self, buf: &[u8], offset: u64) -> io::Result<usize> {
        filelike::write_at(self, buf, offset)
    }

    #[inline]
    fn write_all_at(&mut self, buf: &[u8], offset: u64) -> io::Result<()> {
        filelike::write_all_at(self, buf, offset)
    }

    #[inline]
    fn write_vectored_at(&mut self, bufs: &[IoSlice], offset: u64) -> io::Result<usize> {
        filelike::write_vectored_at(self, bufs, offset)
    }

    #[inline]
    fn write_all_vectored_at(&mut self, bufs: &mut [IoSlice], offset: u64) -> io::Result<()> {
        filelike::write_all_vectored_at(self, bufs, offset)
    }

    #[inline]
    fn is_write_vectored_at(&self) -> bool {
        filelike::is_write_vectored_at(self)
    }

    #[inline]
    fn copy_from<R: ReadAt>(
        &mut self,
        offset: u64,
        input: &R,
        input_offset: u64,
        len: u64,
    ) -> io::Result<u64> {
        filelike::copy_from(self, offset, input, input_offset, len)
    }

    #[inline]
    fn set_len(&mut self, size: u64) -> io::Result<()> {
        filelike::set_len(self, size)
    }
}

impl WriteAt for RangeEditor {
    fn write_at(&mut self, buf: &[u8], offset: u64) -> io::Result<usize> {
        filelike::write_at(self, buf, offset)
    }

    #[inline]
    fn write_all_at(&mut self, buf: &[u8], offset: u64) -> io::Result<()> {
        filelike::write_all_at(self, buf, offset)
    }

    #[inline]
    fn write_vectored_at(&mut self, bufs: &[IoSlice], offset: u64) -> io::Result<usize> {
        filelike::write_vectored_at(self, bufs, offset)
    }

    #[inline]
    fn write_all_vectored_at(&mut self, bufs: &mut [IoSlice], offset: u64) -> io::Result<()> {
        filelike::write_all_vectored_at(self, bufs, offset)
    }

    #[inline]
    fn is_write_vectored_at(&self) -> bool {
        filelike::is_write_vectored_at(self)
    }

    #[inline]
    fn copy_from<R: ReadAt>(
        &mut self,
        offset: u64,
        input: &R,
        input_offset: u64,
        len: u64,
    ) -> io::Result<u64> {
        filelike::copy_from(self, offset, input, input_offset, len)
    }

    #[inline]
    fn set_len(&mut self, size: u64) -> io::Result<()> {
        filelike::set_len(self, size)
    }
}

impl Range for fs::File {
    #[inline]
    fn metadata(&self) -> io::Result<Metadata> {
        filelike::metadata(self)
    }

    #[inline]
    fn advise(&self, offset: u64, len: u64, advice: Advice) -> io::Result<()> {
        filelike::advise(self, offset, len, advice)
    }
}

impl ReadAt for fs::File {
    #[inline]
    fn read_at(&self, buf: &mut [u8], offset: u64) -> io::Result<usize> {
        filelike::read_at(self, buf, offset)
    }

    #[inline]
    fn read_exact_at(&self, buf: &mut [u8], offset: u64) -> io::Result<()> {
        filelike::read_exact_at(self, buf, offset)
    }

    #[inline]
    fn read_vectored_at(&self, bufs: &mut [IoSliceMut], offset: u64) -> io::Result<usize> {
        filelike::read_vectored_at(self, bufs, offset)
    }

    #[inline]
    fn read_exact_vectored_at(&self, bufs: &mut [IoSliceMut], offset: u64) -> io::Result<()> {
        filelike::read_exact_vectored_at(self, bufs, offset)
    }

    #[inline]
    fn is_read_vectored_at(&self) -> bool {
        filelike::is_read_vectored_at(self)
    }

    #[cfg(feature = "io-streams")]
    #[inline]
    fn read_via_stream_at(&self, offset: u64) -> io::Result<StreamReader> {
        filelike::read_via_stream_at(self, offset)
    }
}

impl WriteAt for fs::File {
    fn write_at(&mut self, buf: &[u8], offset: u64) -> io::Result<usize> {
        filelike::write_at(self, buf, offset)
    }

    #[inline]
    fn write_all_at(&mut self, buf: &[u8], offset: u64) -> io::Result<()> {
        filelike::write_all_at(self, buf, offset)
    }

    #[inline]
    fn write_vectored_at(&mut self, bufs: &[IoSlice], offset: u64) -> io::Result<usize> {
        filelike::write_vectored_at(self, bufs, offset)
    }

    #[inline]
    fn write_all_vectored_at(&mut self, bufs: &mut [IoSlice], offset: u64) -> io::Result<()> {
        filelike::write_all_vectored_at(self, bufs, offset)
    }

    #[inline]
    fn is_write_vectored_at(&self) -> bool {
        filelike::is_write_vectored_at(self)
    }

    #[inline]
    fn copy_from<R: ReadAt>(
        &mut self,
        offset: u64,
        input: &R,
        input_offset: u64,
        len: u64,
    ) -> io::Result<u64> {
        filelike::copy_from(self, offset, input, input_offset, len)
    }

    #[inline]
    fn set_len(&mut self, size: u64) -> io::Result<()> {
        filelike::set_len(self, size)
    }
}

#[cfg(feature = "cap-std")]
impl Range for cap_std::fs::File {
    #[inline]
    fn metadata(&self) -> io::Result<Metadata> {
        filelike::metadata(self)
    }

    #[inline]
    fn advise(&self, offset: u64, len: u64, advice: Advice) -> io::Result<()> {
        filelike::advise(self, offset, len, advice)
    }
}

#[cfg(feature = "cap-std")]
impl ReadAt for cap_std::fs::File {
    #[inline]
    fn read_at(&self, buf: &mut [u8], offset: u64) -> io::Result<usize> {
        filelike::read_at(self, buf, offset)
    }

    #[inline]
    fn read_exact_at(&self, buf: &mut [u8], offset: u64) -> io::Result<()> {
        filelike::read_exact_at(self, buf, offset)
    }

    #[inline]
    fn read_vectored_at(&self, bufs: &mut [IoSliceMut], offset: u64) -> io::Result<usize> {
        filelike::read_vectored_at(self, bufs, offset)
    }

    #[inline]
    fn read_exact_vectored_at(&self, bufs: &mut [IoSliceMut], offset: u64) -> io::Result<()> {
        filelike::read_exact_vectored_at(self, bufs, offset)
    }

    #[inline]
    fn is_read_vectored_at(&self) -> bool {
        filelike::is_read_vectored_at(self)
    }

    #[cfg(feature = "io-streams")]
    #[inline]
    fn read_via_stream_at(&self, offset: u64) -> io::Result<StreamReader> {
        filelike::read_via_stream_at(self, offset)
    }
}

#[cfg(feature = "cap-std")]
impl WriteAt for cap_std::fs::File {
    fn write_at(&mut self, buf: &[u8], offset: u64) -> io::Result<usize> {
        filelike::write_at(self, buf, offset)
    }

    #[inline]
    fn write_all_at(&mut self, buf: &[u8], offset: u64) -> io::Result<()> {
        filelike::write_all_at(self, buf, offset)
    }

    #[inline]
    fn write_vectored_at(&mut self, bufs: &[IoSlice], offset: u64) -> io::Result<usize> {
        filelike::write_vectored_at(self, bufs, offset)
    }

    #[inline]
    fn write_all_vectored_at(&mut self, bufs: &mut [IoSlice], offset: u64) -> io::Result<()> {
        filelike::write_all_vectored_at(self, bufs, offset)
    }

    #[inline]
    fn is_write_vectored_at(&self) -> bool {
        filelike::is_write_vectored_at(self)
    }

    #[inline]
    fn copy_from<R: ReadAt>(
        &mut self,
        offset: u64,
        input: &R,
        input_offset: u64,
        len: u64,
    ) -> io::Result<u64> {
        filelike::copy_from(self, offset, input, input_offset, len)
    }

    #[inline]
    fn set_len(&mut self, size: u64) -> io::Result<()> {
        filelike::set_len(self, size)
    }
}

#[cfg(feature = "cap-async-std")]
impl Range for cap_async_std::fs::File {
    #[inline]
    fn metadata(&self) -> io::Result<Metadata> {
        filelike::metadata(self)
    }

    #[inline]
    fn advise(&self, offset: u64, len: u64, advice: Advice) -> io::Result<()> {
        filelike::advise(self, offset, len, advice)
    }
}

#[cfg(feature = "cap-async-std")]
impl ReadAt for cap_async_std::fs::File {
    #[inline]
    fn read_at(&self, buf: &mut [u8], offset: u64) -> io::Result<usize> {
        filelike::read_at(self, buf, offset)
    }

    #[inline]
    fn read_exact_at(&self, buf: &mut [u8], offset: u64) -> io::Result<()> {
        filelike::read_exact_at(self, buf, offset)
    }

    #[inline]
    fn read_vectored_at(&self, bufs: &mut [IoSliceMut], offset: u64) -> io::Result<usize> {
        filelike::read_vectored_at(self, bufs, offset)
    }

    #[inline]
    fn read_exact_vectored_at(&self, bufs: &mut [IoSliceMut], offset: u64) -> io::Result<()> {
        filelike::read_exact_vectored_at(self, bufs, offset)
    }

    #[inline]
    fn is_read_vectored_at(&self) -> bool {
        filelike::is_read_vectored_at(self)
    }

    #[cfg(feature = "io-streams")]
    #[inline]
    fn read_via_stream_at(&self, offset: u64) -> io::Result<StreamReader> {
        filelike::read_via_stream_at(self, offset)
    }
}

#[cfg(feature = "cap-async-std")]
impl WriteAt for cap_async_std::fs::File {
    fn write_at(&mut self, buf: &[u8], offset: u64) -> io::Result<usize> {
        filelike::write_at(self, buf, offset)
    }

    #[inline]
    fn write_all_at(&mut self, buf: &[u8], offset: u64) -> io::Result<()> {
        filelike::write_all_at(self, buf, offset)
    }

    #[inline]
    fn write_vectored_at(&mut self, bufs: &[IoSlice], offset: u64) -> io::Result<usize> {
        filelike::write_vectored_at(self, bufs, offset)
    }

    #[inline]
    fn write_all_vectored_at(&mut self, bufs: &mut [IoSlice], offset: u64) -> io::Result<()> {
        filelike::write_all_vectored_at(self, bufs, offset)
    }

    #[inline]
    fn is_write_vectored_at(&self) -> bool {
        filelike::is_write_vectored_at(self)
    }

    #[inline]
    fn copy_from<R: ReadAt>(
        &mut self,
        offset: u64,
        input: &R,
        input_offset: u64,
        len: u64,
    ) -> io::Result<u64> {
        filelike::copy_from(self, offset, input, input_offset, len)
    }

    #[inline]
    fn set_len(&mut self, size: u64) -> io::Result<()> {
        filelike::set_len(self, size)
    }
}

#[cfg(feature = "async-std")]
impl Range for async_std::fs::File {
    #[inline]
    fn metadata(&self) -> io::Result<Metadata> {
        filelike::metadata(self)
    }

    #[inline]
    fn advise(&self, offset: u64, len: u64, advice: Advice) -> io::Result<()> {
        filelike::advise(self, offset, len, advice)
    }
}

#[cfg(feature = "async-std")]
impl ReadAt for async_std::fs::File {
    #[inline]
    fn read_at(&self, buf: &mut [u8], offset: u64) -> io::Result<usize> {
        filelike::read_at(self, buf, offset)
    }

    #[inline]
    fn read_exact_at(&self, buf: &mut [u8], offset: u64) -> io::Result<()> {
        filelike::read_exact_at(self, buf, offset)
    }

    #[inline]
    fn read_vectored_at(&self, bufs: &mut [IoSliceMut], offset: u64) -> io::Result<usize> {
        filelike::read_vectored_at(self, bufs, offset)
    }

    #[inline]
    fn read_exact_vectored_at(&self, bufs: &mut [IoSliceMut], offset: u64) -> io::Result<()> {
        filelike::read_exact_vectored_at(self, bufs, offset)
    }

    #[inline]
    fn is_read_vectored_at(&self) -> bool {
        filelike::is_read_vectored_at(self)
    }

    #[cfg(feature = "io-streams")]
    #[inline]
    fn read_via_stream_at(&self, offset: u64) -> io::Result<StreamReader> {
        filelike::read_via_stream_at(self, offset)
    }
}

#[cfg(feature = "async-std")]
impl WriteAt for async_std::fs::File {
    fn write_at(&mut self, buf: &[u8], offset: u64) -> io::Result<usize> {
        filelike::write_at(self, buf, offset)
    }

    #[inline]
    fn write_all_at(&mut self, buf: &[u8], offset: u64) -> io::Result<()> {
        filelike::write_all_at(self, buf, offset)
    }

    #[inline]
    fn write_vectored_at(&mut self, bufs: &[IoSlice], offset: u64) -> io::Result<usize> {
        filelike::write_vectored_at(self, bufs, offset)
    }

    #[inline]
    fn write_all_vectored_at(&mut self, bufs: &mut [IoSlice], offset: u64) -> io::Result<()> {
        filelike::write_all_vectored_at(self, bufs, offset)
    }

    #[inline]
    fn is_write_vectored_at(&self) -> bool {
        filelike::is_write_vectored_at(self)
    }

    #[inline]
    fn copy_from<R: ReadAt>(
        &mut self,
        offset: u64,
        input: &R,
        input_offset: u64,
        len: u64,
    ) -> io::Result<u64> {
        filelike::copy_from(self, offset, input, input_offset, len)
    }

    #[inline]
    fn set_len(&mut self, size: u64) -> io::Result<()> {
        filelike::set_len(self, size)
    }
}

#[cfg(not(windows))]
impl AsRawFd for RangeReader {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        self.file.as_raw_fd()
    }
}

#[cfg(windows)]
impl AsRawHandle for RangeReader {
    #[inline]
    fn as_raw_handle(&self) -> RawHandle {
        self.file.as_raw_handle()
    }
}

#[cfg(not(windows))]
impl AsRawFd for RangeWriter {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        self.file.as_raw_fd()
    }
}

#[cfg(windows)]
impl AsRawHandle for RangeWriter {
    #[inline]
    fn as_raw_handle(&self) -> RawHandle {
        self.file.as_raw_handle()
    }
}

#[cfg(not(windows))]
impl AsRawFd for RangeEditor {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        self.file.as_raw_fd()
    }
}

#[cfg(windows)]
impl AsRawHandle for RangeEditor {
    #[inline]
    fn as_raw_handle(&self) -> RawHandle {
        self.file.as_raw_handle()
    }
}

// On Linux, use `memfd_create`.
#[cfg(any(target_os = "android", target_os = "linux"))]
fn create_anonymous() -> io::Result<UnsafeFile> {
    let flags = libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING;
    let name = b"io_ranges anonymous file\0"
        .as_ptr()
        .cast::<libc::c_char>();
    let fd = unsafe { memfd_create(name, flags) };
    if fd == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(UnsafeFile::from_raw_fd(fd))
}

// Otherwise, use a temporary file.
#[cfg(not(any(target_os = "android", target_os = "linux")))]
fn create_anonymous() -> io::Result<UnsafeFile> {
    let file = tempfile::tempfile()?;
    Ok(file.into_unsafe_file())
}

#[cfg(any(target_os = "android", target_os = "linux"))]
unsafe fn memfd_create(name: *const libc::c_char, flags: libc::c_uint) -> libc::c_int {
    libc::syscall(libc::SYS_memfd_create, name, flags) as libc::c_int
}
