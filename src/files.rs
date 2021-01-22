#[cfg(unix)]
use std::os::unix::{
    fs::MetadataExt,
    io::{AsRawFd, RawFd},
};
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
#[cfg(feature = "io-streams")]
use {
    crate::file_streamer::FileStreamer,
    cap_fs_ext::{OpenOptions, Reopen},
    io_streams::StreamReader,
    std::io::SeekFrom,
};

pub use system_interface::fs::Advice;

/// Metadata information about a file.
pub struct Metadata {
    len: u64,
    blksize: u64,
}

#[allow(clippy::len_without_is_empty)]
impl Metadata {
    /// Returns the size of the file, in bytes, this metadata is for.
    #[inline]
    #[must_use]
    pub const fn len(&self) -> u64 {
        self.len
    }

    /// Returns the block size for filesystem I/O.
    #[inline]
    #[must_use]
    pub const fn blksize(&self) -> u64 {
        self.blksize
    }
}

/// A minimal base trait for file I/O. Defines operations common to all kinds
/// of random-access devices that fit the "file" concept, including normal
/// files, block devices, and in-memory buffers.
pub trait MinimalFile: AsUnsafeFile {
    /// Return the `Metadata` for the file. This is similar to
    /// `std::fs::File::metadata`, though it returns fewer fields since the
    /// underlying device may not be an actual filesystem inode.
    #[inline]
    fn metadata(&self) -> io::Result<Metadata> {
        self.as_file_view().metadata().map(|meta| {
            Metadata {
                len: meta.len(),

                #[cfg(not(windows))]
                blksize: meta.blksize(),

                // Windows doesn't have a convenient way to query this, but
                // it often uses this specific value.
                #[cfg(windows)]
                blksize: 0x1000,
            }
        })
    }

    /// Announce the expected access pattern of the data at the given offset.
    #[inline]
    fn advise(&self, offset: u64, len: u64, advice: Advice) -> io::Result<()> {
        self.as_file_view().advise(offset, len, advice)
    }
}

/// A trait for reading from files.
pub trait ReadAt: MinimalFile {
    /// Reads a number of bytes starting from a given offset.
    ///
    /// This is similar to [`std::os::unix::fs::FileExt::read_at`], except it
    /// takes `self` by immutable reference since the entire side effect is
    /// I/O, and it's supported on non-Unix platforms including Windows.
    ///
    /// [`std::os::unix::fs::FileExt::read_at`]: https://doc.rust-lang.org/std/os/unix/fs/trait.FileExt.html#tymethod.read_at
    #[inline]
    fn read_at(&self, buf: &mut [u8], offset: u64) -> io::Result<usize> {
        self.as_file_view().read_at(buf, offset)
    }

    /// Reads the exact number of byte required to fill buf from the given
    /// offset.
    ///
    /// This is similar to [`std::os::unix::fs::FileExt::read_exact_at`], except
    /// it takes `self` by immutable reference since the entire side effect is
    /// I/O, and it's supported on non-Unix platforms including Windows.
    ///
    /// [`std::os::unix::fs::FileExt::read_exact_at`]: https://doc.rust-lang.org/std/os/unix/fs/trait.FileExt.html#tymethod.read_exact_at
    #[inline]
    fn read_exact_at(&self, buf: &mut [u8], offset: u64) -> io::Result<()> {
        self.as_file_view().read_exact_at(buf, offset)
    }

    /// Is to `read_vectored` what `read_at` is to `read`.
    #[inline]
    fn read_vectored_at(&self, bufs: &mut [IoSliceMut], offset: u64) -> io::Result<usize> {
        self.as_file_view().read_vectored_at(bufs, offset)
    }

    /// Is to `read_exact_vectored` what `read_exact_at` is to `read_exact`.
    #[inline]
    fn read_exact_vectored_at(&self, bufs: &mut [IoSliceMut], offset: u64) -> io::Result<()> {
        self.as_file_view().read_exact_vectored_at(bufs, offset)
    }

    /// Determines if `Self` has an efficient `read_vectored_at` implementation.
    #[inline]
    fn is_read_vectored_at(&self) -> bool {
        false
    }

    /// Create a `StreamReader` which reads from the file at the given offset.
    #[cfg(feature = "io-streams")]
    fn read_via_stream(&self, offset: u64) -> io::Result<StreamReader> {
        // On operating systems where we can do so, reopen the file so that we
        // get an independent current position.
        if let Ok(file) = self.as_file_view().reopen(OpenOptions::new().read(true)) {
            if offset != 0 {
                file.seek(SeekFrom::Start(offset))?;
            }
            return Ok(StreamReader::file(file));
        }

        // Otherwise, manually stream the file.
        StreamReader::piped_thread(Box::new(FileStreamer::new(
            self.as_file_view().try_clone()?,
            offset,
        )))
    }
}

/// A trait for writing to files.
pub trait WriteAt: MinimalFile {
    /// Writes a number of bytes starting from a given offset.
    ///
    /// This is similar to [`std::os::unix::fs::FileExt::write_at`], except it
    /// takes `self` by immutable reference since the entire side effect is
    /// I/O, and it's supported on non-Unix platforms including Windows.
    ///
    /// [`std::os::unix::fs::FileExt::write_at`]: https://doc.rust-lang.org/std/os/unix/fs/trait.FileExt.html#tymethod.write_at
    #[inline]
    fn write_at(&self, buf: &[u8], offset: u64) -> io::Result<usize> {
        self.as_file_view().write_at(buf, offset)
    }

    /// Attempts to write an entire buffer starting from a given offset.
    ///
    /// This is similar to [`std::os::unix::fs::FileExt::write_all_at`], except
    /// it takes `self` by immutable reference since the entire side effect is
    /// I/O, and it's supported on non-Unix platforms including Windows.
    ///
    /// [`std::os::unix::fs::FileExt::write_all_at`]: https://doc.rust-lang.org/std/os/unix/fs/trait.FileExt.html#tymethod.write_all_at
    #[inline]
    fn write_all_at(&self, buf: &[u8], offset: u64) -> io::Result<()> {
        self.as_file_view().write_all_at(buf, offset)
    }

    /// Is to `write_vectored` what `write_at` is to `write`.
    #[inline]
    fn write_vectored_at(&self, bufs: &[IoSlice], offset: u64) -> io::Result<usize> {
        self.as_file_view().write_vectored_at(bufs, offset)
    }

    /// Is to `write_all_vectored` what `write_all_at` is to `write_all`.
    #[inline]
    fn write_all_vectored_at(&self, bufs: &mut [IoSlice], offset: u64) -> io::Result<()> {
        self.as_file_view().write_all_vectored_at(bufs, offset)
    }

    /// Allocate space in the file, increasing the file size as needed, and
    /// ensuring that there are no holes under the given range.
    #[inline]
    fn allocate(&self, offset: u64, len: u64) -> io::Result<()> {
        self.as_file_view().allocate(offset, len)
    }

    /// Determines if `Self` has an efficient `write_vectored_at` implementation.
    #[inline]
    fn is_write_vectored_at(&self) -> bool {
        false
    }
}

/// A trait for reading and writing to files.
pub trait EditAt: ReadAt + WriteAt {}

/// A random-access input source.
#[derive(Debug)]
pub struct FileReader {
    unsafe_file: UnsafeFile,
}

/// A random-access output source.
#[derive(Debug)]
pub struct FileWriter {
    unsafe_file: UnsafeFile,
}

/// A random-access input and output source.
#[derive(Debug)]
pub struct FileEditor {
    unsafe_file: UnsafeFile,
}

impl FileReader {
    /// Convert a `File` into a `FileReader`.
    #[inline]
    #[must_use]
    pub fn file<IUF: IntoUnsafeFile + Read + Write + Seek>(file: IUF) -> Self {
        Self {
            unsafe_file: file.into_unsafe_file(),
        }
    }

    /// Copy a slice of bytes into a memory buffer to allow it to be accessed
    /// in the manner of a file.
    #[inline]
    pub fn bytes(bytes: &[u8]) -> io::Result<Self> {
        // On Linux, use `memfd_create`.
        #[cfg(any(target_os = "android", target_os = "linux"))]
        {
            let flags = libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING;
            let name = b"FileReader::bytes\0".as_ptr() as *const libc::c_char;
            let fd = unsafe { memfd_create(name, flags) };
            if fd == -1 {
                return Err(io::Error::last_os_error());
            }
            let file = UnsafeFile::from_raw_fd(fd);
            file.as_file_view().write_all(bytes)?;
            Ok(Self { unsafe_file: file })
        }

        // Otherwise, use a temporary file.
        #[cfg(not(any(target_os = "android", target_os = "linux")))]
        {
            let file = tempfile::tempfile()?;
            file.write_all(bytes)?;
            Ok(Self {
                unsafe_file: file.into_unsafe_file(),
            })
        }
    }
}

impl FileWriter {
    /// Convert a `File` into a `FileWriter`.
    ///
    /// The file must not be opened in [append mode].
    ///
    /// [append mode]: https://doc.rust-lang.org/stable/std/fs/struct.OpenOptions.html#method.append
    #[inline]
    #[must_use]
    pub fn file<IUF: IntoUnsafeFile + Read + Write + Seek>(file: IUF) -> Self {
        Self::_file(file.into_unsafe_file())
    }

    #[inline]
    fn _file(file: UnsafeFile) -> Self {
        // On Linux, `pwrite` on a file opened with `O_APPEND` writes to the
        // end of the file, ignoring the offset.
        #[cfg(not(windows))]
        {
            assert!(
                !posish::fs::getfl(&file)
                    .unwrap()
                    .contains(posish::fs::OFlags::APPEND),
                "FileWriter doesn't support files opened with O_APPEND"
            );
        }
        #[cfg(windows)]
        {
            assert_ne!(
                (winx::query_access_information(file.as_raw_handle())?
                    & AccessMode::FILE_APPEND_DATA)
                    == AccessMode::FILE_APPEND_DATA,
                "FileWriter doesn't support files opened with FILE_APPEND_DATA"
            );
        }

        Self { unsafe_file: file }
    }
}

impl FileEditor {
    /// Convert a `File` into a `FileEditor`.
    #[inline]
    #[must_use]
    pub fn file<IUF: IntoUnsafeFile + Read + Write + Seek>(file: IUF) -> Self {
        Self {
            unsafe_file: file.into_unsafe_file(),
        }
    }
}

impl Drop for FileReader {
    #[inline]
    fn drop(&mut self) {
        drop(unsafe { fs::File::from_unsafe_file(self.unsafe_file) })
    }
}

impl Drop for FileWriter {
    #[inline]
    fn drop(&mut self) {
        drop(unsafe { fs::File::from_unsafe_file(self.unsafe_file) })
    }
}

impl Drop for FileEditor {
    #[inline]
    fn drop(&mut self) {
        drop(unsafe { fs::File::from_unsafe_file(self.unsafe_file) })
    }
}

impl MinimalFile for FileReader {}
impl MinimalFile for FileWriter {}
impl MinimalFile for FileEditor {}

impl ReadAt for FileReader {}
impl ReadAt for FileEditor {}

impl WriteAt for FileWriter {}
impl WriteAt for FileEditor {}

#[cfg(not(windows))]
impl AsRawFd for FileReader {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        self.unsafe_file.as_raw_fd()
    }
}

#[cfg(windows)]
impl AsRawHandle for FileReader {
    #[inline]
    fn as_raw_handle(&self) -> RawHandle {
        self.unsafe_file.as_raw_handle()
    }
}

#[cfg(not(windows))]
impl AsRawFd for FileWriter {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        self.unsafe_file.as_raw_fd()
    }
}

#[cfg(windows)]
impl AsRawHandle for FileWriter {
    #[inline]
    fn as_raw_handle(&self) -> RawHandle {
        self.unsafe_file.as_raw_handle()
    }
}

#[cfg(not(windows))]
impl AsRawFd for FileEditor {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        self.unsafe_file.as_raw_fd()
    }
}

#[cfg(windows)]
impl AsRawHandle for FileEditor {
    #[inline]
    fn as_raw_handle(&self) -> RawHandle {
        self.unsafe_file.as_raw_handle()
    }
}

#[cfg(any(target_os = "android", target_os = "linux"))]
unsafe fn memfd_create(name: *const libc::c_char, flags: libc::c_uint) -> libc::c_int {
    libc::syscall(libc::SYS_memfd_create, name, flags) as libc::c_int
}
