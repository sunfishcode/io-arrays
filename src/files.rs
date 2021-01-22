use crate::borrow_streamer::BorrowStreamer;
#[cfg(unix)]
use std::os::unix::{
    fs::MetadataExt,
    io::{AsRawFd, RawFd},
};
#[cfg(target_os = "wasi")]
use std::os::wasi::io::{AsRawFd, RawFd};
use std::{
    fs,
    io::{self, copy, IoSlice, IoSliceMut, Read, Seek, Write},
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
#[cfg(windows)]
use {
    crate::windows,
    std::os::windows::io::{AsRawHandle, RawHandle},
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
        <fs::File as FileIoExt>::advise(&self.as_file_view(), offset, len, advice)
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
        <fs::File as FileIoExt>::read_at(&self.as_file_view(), buf, offset)
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
        <fs::File as FileIoExt>::read_exact_at(&self.as_file_view(), buf, offset)
    }

    /// Is to `read_vectored` what `read_at` is to `read`.
    #[inline]
    fn read_vectored_at(&self, bufs: &mut [IoSliceMut], offset: u64) -> io::Result<usize> {
        <fs::File as FileIoExt>::read_vectored_at(&self.as_file_view(), bufs, offset)
    }

    /// Is to `read_exact_vectored` what `read_exact_at` is to `read_exact`.
    #[inline]
    fn read_exact_vectored_at(&self, bufs: &mut [IoSliceMut], offset: u64) -> io::Result<()> {
        <fs::File as FileIoExt>::read_exact_vectored_at(&self.as_file_view(), bufs, offset)
    }

    /// Determines if `Self` has an efficient `read_vectored_at` implementation.
    #[inline]
    fn is_read_vectored_at(&self) -> bool {
        <fs::File as FileIoExt>::is_read_vectored_at(&self.as_file_view())
    }

    /// Read all bytes until EOF in this source, placing them into `buf`.
    #[inline]
    fn read_to_end_at(&self, buf: &mut Vec<u8>, offset: u64) -> io::Result<usize> {
        <fs::File as FileIoExt>::read_to_end_at(&self.as_file_view(), buf, offset)
    }

    /// Read all bytes until EOF in this source, appending them to `buf`.
    #[inline]
    fn read_to_string_at(&self, buf: &mut String, offset: u64) -> io::Result<usize> {
        <fs::File as FileIoExt>::read_to_string_at(&self.as_file_view(), buf, offset)
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
        <fs::File as FileIoExt>::write_at(&self.as_file_view(), buf, offset)
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
        <fs::File as FileIoExt>::write_all_at(&self.as_file_view(), buf, offset)
    }

    /// Is to `write_vectored` what `write_at` is to `write`.
    #[inline]
    fn write_vectored_at(&self, bufs: &[IoSlice], offset: u64) -> io::Result<usize> {
        <fs::File as FileIoExt>::write_vectored_at(&self.as_file_view(), bufs, offset)
    }

    /// Is to `write_all_vectored` what `write_all_at` is to `write_all`.
    #[inline]
    fn write_all_vectored_at(&self, bufs: &mut [IoSlice], offset: u64) -> io::Result<()> {
        <fs::File as FileIoExt>::write_all_vectored_at(&self.as_file_view(), bufs, offset)
    }

    /// Allocate space in the file, increasing the file size as needed, and
    /// ensuring that there are no holes under the given range.
    #[inline]
    fn allocate(&self, offset: u64, len: u64) -> io::Result<()> {
        <fs::File as FileIoExt>::allocate(&self.as_file_view(), offset, len)
    }

    /// Determines if `Self` has an efficient `write_vectored_at` implementation.
    #[inline]
    fn is_write_vectored_at(&self) -> bool {
        <fs::File as FileIoExt>::is_write_vectored_at(&self.as_file_view())
    }

    /// Copy `len` bytes from `input` at `input_offset` to `self` at `offset`.
    #[inline]
    fn copy_from<R: ReadAt>(
        &self,
        offset: u64,
        input: &R,
        input_offset: u64,
        len: u64,
    ) -> io::Result<u64> {
        let output_view = self.as_file_view();
        let input_view = input.as_file_view();
        let mut output_streamer = BorrowStreamer::new(&output_view, offset);
        let input_streamer = BorrowStreamer::new(&input_view, input_offset);
        copy(&mut input_streamer.take(len), &mut output_streamer)
    }
}

/// A trait for reading and writing to files.
pub trait EditAt: ReadAt + WriteAt {}

/// A random-access input source.
#[derive(Debug)]
pub struct FileReader {
    file: fs::File,
}

/// A random-access output source.
#[derive(Debug)]
pub struct FileWriter {
    file: fs::File,
}

/// A random-access input and output source.
#[derive(Debug)]
pub struct FileEditor {
    file: fs::File,
}

impl FileReader {
    /// Convert a `File` into a `FileReader`.
    #[inline]
    #[must_use]
    pub fn file<Filelike: IntoUnsafeFile + Read + Seek>(filelike: Filelike) -> Self {
        Self {
            file: fs::File::from_filelike(filelike),
        }
    }

    /// Copy a slice of bytes into a memory buffer to allow it to be accessed
    /// in the manner of a file.
    #[inline]
    pub fn bytes(bytes: &[u8]) -> io::Result<Self> {
        let unsafe_file = create_anonymous()?;
        unsafe_file.as_file_view().write_all(bytes)?;
        Ok(Self {
            file: unsafe { fs::File::from_unsafe_file(unsafe_file) },
        })
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
                "FileWriter doesn't support files opened with O_APPEND"
            );
        }
        #[cfg(windows)]
        {
            assert!(
                (winx::file::query_access_information(file.as_raw_handle()).unwrap()
                    & winx::file::AccessMode::FILE_APPEND_DATA)
                    == winx::file::AccessMode::FILE_APPEND_DATA,
                "FileWriter doesn't support files opened with FILE_APPEND_DATA"
            );
        }

        Self { file }
    }
}

impl FileEditor {
    /// Convert a `File` into a `FileEditor`.
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

impl MinimalFile for FileReader {}
impl MinimalFile for FileWriter {}
impl MinimalFile for FileEditor {}

// We can't use Windows' `read_at` or `write_at` here because it isn't able to
// extend the length of a file we can't `reopen` (such as temporary files).
// However, while `FileIoExt` can't use `seek_write` because it mutates the
// current position, here we *can* use plain `seek_write` because `FileEditor`
// doesn't expose the current position.
#[cfg(not(windows))]
impl ReadAt for FileReader {}
#[cfg(not(windows))]
impl ReadAt for FileEditor {}
#[cfg(not(windows))]
impl WriteAt for FileWriter {}
#[cfg(not(windows))]
impl WriteAt for FileEditor {}

impl MinimalFile for fs::File {}
impl ReadAt for fs::File {}
impl WriteAt for fs::File {}

#[cfg(feature = "cap-std")]
impl MinimalFile for cap_std::fs::File {}
#[cfg(feature = "cap-std")]
impl ReadAt for cap_std::fs::File {}
#[cfg(feature = "cap-std")]
impl WriteAt for cap_std::fs::File {}

#[cfg(feature = "cap-async-std")]
impl MinimalFile for cap_async_std::fs::File {}
#[cfg(feature = "cap-async-std")]
impl ReadAt for cap_async_std::fs::File {}
#[cfg(feature = "cap-async-std")]
impl WriteAt for cap_async_std::fs::File {}

#[cfg(feature = "async-std")]
impl MinimalFile for async_std::fs::File {}
#[cfg(feature = "async-std")]
impl ReadAt for async_std::fs::File {}
#[cfg(feature = "async-std")]
impl WriteAt for async_std::fs::File {}

#[cfg(windows)]
impl ReadAt for FileReader {
    #[inline]
    fn read_at(&self, buf: &mut [u8], offset: u64) -> io::Result<usize> {
        windows::read_at(&self.as_file_view(), buf, offset)
    }

    fn read_exact_at(&self, buf: &mut [u8], offset: u64) -> io::Result<()> {
        windows::read_exact_at(&self.as_file_view(), buf, offset)
    }

    fn read_vectored_at(&self, bufs: &mut [IoSliceMut], offset: u64) -> io::Result<usize> {
        windows::read_vectored_at(&self.as_file_view(), bufs, offset)
    }

    fn read_exact_vectored_at(&self, bufs: &mut [IoSliceMut], offset: u64) -> io::Result<()> {
        windows::read_exact_vectored_at(&self.as_file_view(), bufs, offset)
    }

    fn is_read_vectored_at(&self) -> bool {
        windows::is_read_vectored_at(&self.as_file_view())
    }
}

#[cfg(windows)]
impl ReadAt for FileEditor {
    #[inline]
    fn read_at(&self, buf: &mut [u8], offset: u64) -> io::Result<usize> {
        windows::read_at(&self.as_file_view(), buf, offset)
    }

    fn read_exact_at(&self, buf: &mut [u8], offset: u64) -> io::Result<()> {
        windows::read_exact_at(&self.as_file_view(), buf, offset)
    }

    fn read_vectored_at(&self, bufs: &mut [IoSliceMut], offset: u64) -> io::Result<usize> {
        windows::read_vectored_at(&self.as_file_view(), bufs, offset)
    }

    fn read_exact_vectored_at(&self, bufs: &mut [IoSliceMut], offset: u64) -> io::Result<()> {
        windows::read_exact_vectored_at(&self.as_file_view(), bufs, offset)
    }

    fn is_read_vectored_at(&self) -> bool {
        windows::is_read_vectored_at(&self.as_file_view())
    }
}

#[cfg(windows)]
impl WriteAt for FileWriter {
    #[inline]
    fn write_at(&self, buf: &[u8], offset: u64) -> io::Result<usize> {
        windows::write_at(&self.as_file_view(), buf, offset)
    }

    fn write_all_at(&self, buf: &[u8], offset: u64) -> io::Result<()> {
        windows::write_all_at(&self.as_file_view(), buf, offset)
    }

    fn write_vectored_at(&self, bufs: &[IoSlice], offset: u64) -> io::Result<usize> {
        windows::write_vectored_at(&self.as_file_view(), bufs, offset)
    }

    fn write_all_vectored_at(&self, bufs: &mut [IoSlice], offset: u64) -> io::Result<()> {
        windows::write_all_vectored_at(&self.as_file_view(), bufs, offset)
    }

    #[inline]
    fn allocate(&self, offset: u64, len: u64) -> io::Result<()> {
        windows::allocate(&self.as_file_view(), offset, len)
    }

    #[inline]
    fn is_write_vectored_at(&self) -> bool {
        windows::is_write_vectored_at(&self.as_file_view())
    }
}

#[cfg(windows)]
impl WriteAt for FileEditor {
    #[inline]
    fn write_at(&self, buf: &[u8], offset: u64) -> io::Result<usize> {
        windows::write_at(&self.as_file_view(), buf, offset)
    }

    fn write_all_at(&self, buf: &[u8], offset: u64) -> io::Result<()> {
        windows::write_all_at(&self.as_file_view(), buf, offset)
    }

    fn write_vectored_at(&self, bufs: &[IoSlice], offset: u64) -> io::Result<usize> {
        windows::write_vectored_at(&self.as_file_view(), bufs, offset)
    }

    fn write_all_vectored_at(&self, bufs: &mut [IoSlice], offset: u64) -> io::Result<()> {
        windows::write_all_vectored_at(&self.as_file_view(), bufs, offset)
    }

    #[inline]
    fn allocate(&self, offset: u64, len: u64) -> io::Result<()> {
        windows::allocate(&self.as_file_view(), offset, len)
    }

    #[inline]
    fn is_write_vectored_at(&self) -> bool {
        windows::is_write_vectored_at(&self.as_file_view())
    }
}

#[cfg(not(windows))]
impl AsRawFd for FileReader {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        self.file.as_raw_fd()
    }
}

#[cfg(windows)]
impl AsRawHandle for FileReader {
    #[inline]
    fn as_raw_handle(&self) -> RawHandle {
        self.file.as_raw_handle()
    }
}

#[cfg(not(windows))]
impl AsRawFd for FileWriter {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        self.file.as_raw_fd()
    }
}

#[cfg(windows)]
impl AsRawHandle for FileWriter {
    #[inline]
    fn as_raw_handle(&self) -> RawHandle {
        self.file.as_raw_handle()
    }
}

#[cfg(not(windows))]
impl AsRawFd for FileEditor {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        self.file.as_raw_fd()
    }
}

#[cfg(windows)]
impl AsRawHandle for FileEditor {
    #[inline]
    fn as_raw_handle(&self) -> RawHandle {
        self.file.as_raw_handle()
    }
}

// On Linux, use `memfd_create`.
#[cfg(any(target_os = "android", target_os = "linux"))]
fn create_anonymous() -> io::Result<UnsafeFile> {
    let flags = libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING;
    let name = b"io_files anonymous file\0".as_ptr() as *const libc::c_char;
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
