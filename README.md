<div align="center">
  <h1><code>io-ranges</code></h1>

  <p>
    <strong>Random-access I/O</strong>
  </p>

  <p>
    <a href="https://github.com/sunfishcode/io-ranges/actions?query=workflow%3ACI"><img src="https://github.com/sunfishcode/io-ranges/workflows/CI/badge.svg" alt="Github Actions CI Status" /></a>
    <a href="https://crates.io/crates/io_ranges"><img src="https://img.shields.io/crates/v/io_ranges.svg" alt="crates.io page" /></a>
    <a href="https://docs.rs/io-ranges"><img src="https://docs.rs/io-ranges/badge.svg" alt="docs.rs docs" /></a>
  </p>
</div>

This crate defines [`ReadAt`], [`WriteAt`], and [`EditAt`] traits which define
interfaces to random-access or seekable devices, such as normal files, block
devices, disk partitions, and memory buffers.

It also defines [`RangeReader`], [`RangeWriter`], and [`RangeEditor`] types which
implement the above traits and and can be constructed from any file-like type.
On Posix-ish platforms, including limited support for WASI, these types just
contain a single file descriptor (and implement [`AsRawFd`]), plus any
resources needed to safely hold the file descriptor live. On Windows, they
contain a single file handle (and implement [`AsRawHandle`]).

[`ReadAt`]: https://docs.rs/io-ranges/latest/io_ranges/trait.ReadAt.html
[`WriteAt`]: https://docs.rs/io-ranges/latest/io_ranges/trait.WriteAt.html
[`EditAt`]: https://docs.rs/io-ranges/latest/io_ranges/trait.EditAt.html
[`RangeReader`]: https://docs.rs/io-ranges/latest/io_ranges/struct.RangeReader.html
[`RangeWriter`]: https://docs.rs/io-ranges/latest/io_ranges/struct.RangeWriter.html
[`RangeEditor`]: https://docs.rs/io-ranges/latest/io_ranges/struct.RangeEditor.html
[`AsRawFd`]: https://doc.rust-lang.org/std/os/unix/io/trait.AsRawFd.html
[`AsRawHandle`]: https://doc.rust-lang.org/std/os/windows/io/trait.AsRawHandle.html
