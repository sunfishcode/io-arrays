use crate::{filelike, Advice};
#[cfg(not(any(target_os = "android", target_os = "linux")))]
use io_lifetimes::OwnedFilelike;
#[cfg(not(windows))]
use io_lifetimes::{AsFd, BorrowedFd};
use io_lifetimes::{FromFilelike, IntoFilelike};
#[cfg(feature = "io-streams")]
use io_streams::StreamReader;
#[cfg(unix)]
use std::os::unix::io::{AsRawFd, RawFd};
#[cfg(target_os = "wasi")]
use std::os::wasi::io::{AsRawFd, RawFd};
use std::{
    fs,
    io::{self, IoSlice, IoSliceMut, Read, Seek, Write},
};
use system_interface::fs::FileIoExt;
#[cfg(windows)]
use {
    io_lifetimes::{AsHandle, BorrowedHandle},
    std::os::windows::io::{AsRawHandle, RawHandle},
    io_extras::os::windows::{AsRawHandleOrSocket, RawHandleOrSocket},
};

/// Metadata information about an array.
///
/// This is somewhat analogous to [`std::fs::Metadata`], however it only
/// includes a few fields, since arrays are more abstract than files.
pub struct Metadata {
    pub(crate) len: u64,
    pub(crate) blksize: u64,
}

#[allow(clippy::len_without_is_empty)]
impl Metadata {
    /// Returns the size of the array, in bytes, this metadata is for.
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

/// A minimal base trait for array I/O. Defines operations common to all kinds
/// of random-access devices that fit the "array" concept, including normal
/// files, block devices, and in-memory buffers.
///
/// This is a base trait that [`ReadAt`], [`WriteAt`], and [`EditAt`] all
/// share.
pub trait Array {
    /// Return the [`Metadata`] for the array. This is similar to
    /// [`std::fs::File::metadata`], though it returns fewer fields since the
    /// underlying device may not be an actual filesystem inode.
    fn metadata(&self) -> io::Result<Metadata>;

    /// Announce the expected access pattern of the data at the given offset.
    ///
    /// This is purely a performance hint and has no semantic effect.
    fn advise(&self, offset: u64, len: u64, advice: Advice) -> io::Result<()>;
}

/// A trait for reading from arrays.
///
/// This is similar to [`std::io::Read`] except all of the reading functions
/// take an `offset` parameter, specifying a position in the array to read at.
///
/// Unlike `std::io::Read`, `ReadAt`'s functions take a `&self` rather than a
/// `&mut self`, since they don't have a current position to mutate.
pub trait ReadAt: Array {
    /// Reads a number of bytes starting from a given offset.
    ///
    /// This is similar to [`std::os::unix::fs::FileExt::read_at`], except it
    /// takes `self` by immutable reference since the entire side effect is
    /// I/O, and it's supported on non-Unix platforms including Windows.
    ///
    /// [`std::os::unix::fs::FileExt::read_at`]: https://doc.rust-lang.org/std/os/unix/fs/trait.FileExt.html#tymethod.read_at
    fn read_at(&self, buf: &mut [u8], offset: u64) -> io::Result<usize>;

    /// Reads the exact number of byte required to fill `buf` from the given
    /// offset.
    ///
    /// This is similar to [`std::os::unix::fs::FileExt::read_exact_at`],
    /// except it takes `self` by immutable reference since the entire side
    /// effect is I/O, and it's supported on non-Unix platforms including
    /// Windows.
    ///
    /// [`std::os::unix::fs::FileExt::read_exact_at`]: https://doc.rust-lang.org/std/os/unix/fs/trait.FileExt.html#tymethod.read_exact_at
    fn read_exact_at(&self, buf: &mut [u8], offset: u64) -> io::Result<()>;

    /// Is to `read_vectored` what `read_at` is to `read`.
    fn read_vectored_at(&self, bufs: &mut [IoSliceMut], offset: u64) -> io::Result<usize>;

    /// Is to `read_exact_vectored` what `read_exact_at` is to `read_exact`.
    fn read_exact_vectored_at(&self, bufs: &mut [IoSliceMut], offset: u64) -> io::Result<()>;

    /// Determines if `Self` has an efficient `read_vectored_at`
    /// implementation.
    fn is_read_vectored_at(&self) -> bool;

    /// Create a `StreamReader` which reads from the array at the given offset.
    #[cfg(feature = "io-streams")]
    fn read_via_stream_at(&self, offset: u64) -> io::Result<StreamReader>;
}

/// A trait for writing to arrays.
///
/// This is similar to [`std::io::Write`] except all of the reading functions
/// take an `offset` parameter, specifying a position in the array to read at.
pub trait WriteAt: Array {
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

    /// Determines if `Self` has an efficient `write_vectored_at`
    /// implementation.
    fn is_write_vectored_at(&self) -> bool;

    /// Copy `len` bytes from `input` at `input_offset` to `self` at `offset`.
    fn copy_from<R: ReadAt>(
        &mut self,
        offset: u64,
        input: &R,
        input_offset: u64,
        len: u64,
    ) -> io::Result<u64>;

    /// Truncates or extends the underlying array, updating the size of this
    /// array to become `size`.
    fn set_len(&mut self, size: u64) -> io::Result<()>;
}

/// A trait for reading and writing to arrays.
///
/// This trait simply combines [`ReadAt`] and [`WriteAt`] and has a blanket
/// implementation for any type that implements both.
pub trait EditAt: ReadAt + WriteAt {}

impl<T: ReadAt + WriteAt> EditAt for T {}

/// A random-access input source.
#[derive(Debug)]
pub struct ArrayReader {
    file: fs::File,
}

/// A random-access output sink.
#[derive(Debug)]
pub struct ArrayWriter {
    file: fs::File,
}

/// A random-access input source and output sink.
#[derive(Debug)]
pub struct ArrayEditor {
    file: fs::File,
}

impl ArrayReader {
    /// Convert a `File` into a `ArrayReader`.
    #[inline]
    #[must_use]
    pub fn file<Filelike: IntoFilelike + Read + Seek>(filelike: Filelike) -> Self {
        Self {
            file: fs::File::from_into_filelike(filelike),
        }
    }

    /// Copy a slice of bytes into a memory buffer to allow it to be accessed
    /// in the manner of an array.
    #[inline]
    pub fn bytes(bytes: &[u8]) -> io::Result<Self> {
        let owned = create_anonymous()?;
        let file = fs::File::from_into_filelike(owned);
        file.write_all(bytes)?;
        Ok(Self { file })
    }
}

impl ArrayWriter {
    /// Convert a `File` into a `ArrayWriter`.
    ///
    /// The file must not be opened in [append mode].
    ///
    /// [append mode]: https://doc.rust-lang.org/stable/std/fs/struct.OpenOptions.html#method.append
    #[inline]
    #[must_use]
    pub fn file<Filelike: IntoFilelike + Write + Seek>(filelike: Filelike) -> Self {
        Self::_file(fs::File::from_into_filelike(filelike))
    }

    #[inline]
    fn _file(file: fs::File) -> Self {
        // On Linux, `pwrite` on a file opened with `O_APPEND` writes to the
        // end of the file, ignoring the offset.
        #[cfg(not(windows))]
        {
            assert!(
                !rustix::fs::fcntl_getfl(&file)
                    .unwrap()
                    .contains(rustix::fs::OFlags::APPEND),
                "ArrayWriter doesn't support files opened with O_APPEND"
            );
        }
        #[cfg(windows)]
        {
            assert!(
                (winx::file::query_access_information(file.as_handle()).unwrap()
                    & winx::file::AccessMode::FILE_APPEND_DATA)
                    == winx::file::AccessMode::FILE_APPEND_DATA,
                "ArrayWriter doesn't support files opened with FILE_APPEND_DATA"
            );
        }

        Self { file }
    }
}

impl ArrayEditor {
    /// Convert a `File` into a `ArrayEditor`.
    #[inline]
    #[must_use]
    pub fn file<Filelike: IntoFilelike + Read + Write + Seek>(filelike: Filelike) -> Self {
        Self {
            file: fs::File::from_into_filelike(filelike),
        }
    }

    /// Create a temporary anonymous resource which can be accessed in the
    /// manner of an array.
    #[inline]
    pub fn anonymous() -> io::Result<Self> {
        let owned = create_anonymous()?;
        Ok(Self {
            file: fs::File::from_into_filelike(owned),
        })
    }
}

impl Array for ArrayReader {
    #[inline]
    fn metadata(&self) -> io::Result<Metadata> {
        filelike::metadata(self)
    }

    #[inline]
    fn advise(&self, offset: u64, len: u64, advice: Advice) -> io::Result<()> {
        filelike::advise(self, offset, len, advice)
    }
}

impl Array for ArrayWriter {
    #[inline]
    fn metadata(&self) -> io::Result<Metadata> {
        filelike::metadata(self)
    }

    #[inline]
    fn advise(&self, offset: u64, len: u64, advice: Advice) -> io::Result<()> {
        filelike::advise(self, offset, len, advice)
    }
}

impl Array for ArrayEditor {
    #[inline]
    fn metadata(&self) -> io::Result<Metadata> {
        filelike::metadata(self)
    }

    #[inline]
    fn advise(&self, offset: u64, len: u64, advice: Advice) -> io::Result<()> {
        filelike::advise(self, offset, len, advice)
    }
}

impl ReadAt for ArrayReader {
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

impl ReadAt for ArrayEditor {
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

impl WriteAt for ArrayWriter {
    #[inline]
    fn write_at(&mut self, buf: &[u8], offset: u64) -> io::Result<usize> {
        filelike::write_at(&*self, buf, offset)
    }

    #[inline]
    fn write_all_at(&mut self, buf: &[u8], offset: u64) -> io::Result<()> {
        filelike::write_all_at(&*self, buf, offset)
    }

    #[inline]
    fn write_vectored_at(&mut self, bufs: &[IoSlice], offset: u64) -> io::Result<usize> {
        filelike::write_vectored_at(&*self, bufs, offset)
    }

    #[inline]
    fn write_all_vectored_at(&mut self, bufs: &mut [IoSlice], offset: u64) -> io::Result<()> {
        filelike::write_all_vectored_at(&*self, bufs, offset)
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
        filelike::copy_from(&*self, offset, input, input_offset, len)
    }

    #[inline]
    fn set_len(&mut self, size: u64) -> io::Result<()> {
        filelike::set_len(&*self, size)
    }
}

impl WriteAt for ArrayEditor {
    #[inline]
    fn write_at(&mut self, buf: &[u8], offset: u64) -> io::Result<usize> {
        filelike::write_at(&*self, buf, offset)
    }

    #[inline]
    fn write_all_at(&mut self, buf: &[u8], offset: u64) -> io::Result<()> {
        filelike::write_all_at(&*self, buf, offset)
    }

    #[inline]
    fn write_vectored_at(&mut self, bufs: &[IoSlice], offset: u64) -> io::Result<usize> {
        filelike::write_vectored_at(&*self, bufs, offset)
    }

    #[inline]
    fn write_all_vectored_at(&mut self, bufs: &mut [IoSlice], offset: u64) -> io::Result<()> {
        filelike::write_all_vectored_at(&*self, bufs, offset)
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
        filelike::copy_from(&*self, offset, input, input_offset, len)
    }

    #[inline]
    fn set_len(&mut self, size: u64) -> io::Result<()> {
        filelike::set_len(&*self, size)
    }
}

impl Array for fs::File {
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
    #[inline]
    fn write_at(&mut self, buf: &[u8], offset: u64) -> io::Result<usize> {
        filelike::write_at(&*self, buf, offset)
    }

    #[inline]
    fn write_all_at(&mut self, buf: &[u8], offset: u64) -> io::Result<()> {
        filelike::write_all_at(&*self, buf, offset)
    }

    #[inline]
    fn write_vectored_at(&mut self, bufs: &[IoSlice], offset: u64) -> io::Result<usize> {
        filelike::write_vectored_at(&*self, bufs, offset)
    }

    #[inline]
    fn write_all_vectored_at(&mut self, bufs: &mut [IoSlice], offset: u64) -> io::Result<()> {
        filelike::write_all_vectored_at(&*self, bufs, offset)
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
        filelike::copy_from(&*self, offset, input, input_offset, len)
    }

    #[inline]
    fn set_len(&mut self, size: u64) -> io::Result<()> {
        filelike::set_len(&*self, size)
    }
}

#[cfg(feature = "cap-std")]
impl Array for cap_std::fs::File {
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
    #[inline]
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
impl Array for cap_async_std::fs::File {
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
    #[inline]
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
impl Array for async_std::fs::File {
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
    #[inline]
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
impl AsRawFd for ArrayReader {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        self.file.as_raw_fd()
    }
}

#[cfg(not(windows))]
impl AsFd for ArrayReader {
    #[inline]
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.file.as_fd()
    }
}

#[cfg(windows)]
impl AsRawHandle for ArrayReader {
    #[inline]
    fn as_raw_handle(&self) -> RawHandle {
        self.file.as_raw_handle()
    }
}

#[cfg(windows)]
impl AsHandle for ArrayReader {
    #[inline]
    fn as_handle(&self) -> BorrowedHandle<'_> {
        self.file.as_handle()
    }
}

#[cfg(windows)]
impl AsRawHandleOrSocket for ArrayReader {
    #[inline]
    fn as_raw_handle_or_socket(&self) -> RawHandleOrSocket {
        self.file.as_raw_handle_or_socket()
    }
}

#[cfg(not(windows))]
impl AsRawFd for ArrayWriter {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        self.file.as_raw_fd()
    }
}

#[cfg(not(windows))]
impl AsFd for ArrayWriter {
    #[inline]
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.file.as_fd()
    }
}

#[cfg(windows)]
impl AsRawHandle for ArrayWriter {
    #[inline]
    fn as_raw_handle(&self) -> RawHandle {
        self.file.as_raw_handle()
    }
}

#[cfg(windows)]
impl AsHandle for ArrayWriter {
    #[inline]
    fn as_handle(&self) -> BorrowedHandle<'_> {
        self.file.as_handle()
    }
}

#[cfg(windows)]
impl AsRawHandleOrSocket for ArrayWriter {
    #[inline]
    fn as_raw_handle_or_socket(&self) -> RawHandleOrSocket {
        self.file.as_raw_handle_or_socket()
    }
}

#[cfg(not(windows))]
impl AsRawFd for ArrayEditor {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        self.file.as_raw_fd()
    }
}

#[cfg(not(windows))]
impl AsFd for ArrayEditor {
    #[inline]
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.file.as_fd()
    }
}

#[cfg(windows)]
impl AsRawHandle for ArrayEditor {
    #[inline]
    fn as_raw_handle(&self) -> RawHandle {
        self.file.as_raw_handle()
    }
}

#[cfg(windows)]
impl AsHandle for ArrayEditor {
    #[inline]
    fn as_handle(&self) -> BorrowedHandle<'_> {
        self.file.as_handle()
    }
}

#[cfg(windows)]
impl AsRawHandleOrSocket for ArrayEditor {
    #[inline]
    fn as_raw_handle_or_socket(&self) -> RawHandleOrSocket {
        self.file.as_raw_handle_or_socket()
    }
}

// On Linux, use `memfd_create`.
#[cfg(any(target_os = "android", target_os = "linux"))]
fn create_anonymous() -> io::Result<rustix::io::OwnedFd> {
    let flags = rustix::fs::MemfdFlags::CLOEXEC | rustix::fs::MemfdFlags::ALLOW_SEALING;
    let name = cstr::cstr!("io_arrays anonymous file");
    Ok(rustix::fs::memfd_create(name, flags)?)
}

// Otherwise, use a temporary file.
#[cfg(not(any(target_os = "android", target_os = "linux")))]
fn create_anonymous() -> io::Result<OwnedFilelike> {
    let file = tempfile::tempfile()?;
    Ok(file.into_filelike())
}
