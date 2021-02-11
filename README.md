<div align="center">
  <h1><code>io-arrays</code></h1>

  <p>
    <strong>Random-access I/O</strong>
  </p>

  <p>
    <a href="https://github.com/sunfishcode/io-arrays/actions?query=workflow%3ACI"><img src="https://github.com/sunfishcode/io-arrays/workflows/CI/badge.svg" alt="Github Actions CI Status" /></a>
    <a href="https://crates.io/crates/io-arrays"><img src="https://img.shields.io/crates/v/io-arrays.svg" alt="crates.io page" /></a>
    <a href="https://docs.rs/io-arrays"><img src="https://docs.rs/io-arrays/badge.svg" alt="docs.rs docs" /></a>
  </p>
</div>

Think of an *I/O array* as a `Vec<u8>` which lives outside the program. You can
index into it and copy data in and out. You can append to it or shrink it from
the back.

In I/O terms, an I/O array is an abstraction of a "file". It supports doing I/O
at arbitrary offsets, appending, and truncating. You can read from it in a
streaming fashion using [`ReadAt::read_via_stream_at`].

TODO: Writing to an array in a streaming fashion is not yet supported.

This crate defines [`ReadAt`], [`WriteAt`], and [`EditAt`] traits which define
interfaces to random-access or seekable devices, such as normal files, block
devices, disk partitions, and memory buffers.

It also defines [`ArrayReader`], [`ArrayWriter`], and [`ArrayEditor`] types which
implement the above traits and and can be constructed from any file-like type.
On Posix-ish platforms, with support for WASI in development, these types just
contain a single file descriptor (and implement [`AsRawFd`]), plus any
resources needed to safely hold the file descriptor live. On Windows, they
contain a single file handle (and implement [`AsRawHandle`]).

[`ReadAt`]: https://docs.rs/io-arrays/latest/io_arrays/trait.ReadAt.html
[`WriteAt`]: https://docs.rs/io-arrays/latest/io_arrays/trait.WriteAt.html
[`EditAt`]: https://docs.rs/io-arrays/latest/io_arrays/trait.EditAt.html
[`ArrayReader`]: https://docs.rs/io-arrays/latest/io_arrays/struct.ArrayReader.html
[`ArrayWriter`]: https://docs.rs/io-arrays/latest/io_arrays/struct.ArrayWriter.html
[`ArrayEditor`]: https://docs.rs/io-arrays/latest/io_arrays/struct.ArrayEditor.html
[`AsRawFd`]: https://doc.rust-lang.org/std/os/unix/io/trait.AsRawFd.html
[`AsRawHandle`]: https://doc.rust-lang.org/std/os/windows/io/trait.AsRawHandle.html
[`ReadAt::read_via_stream_at`]: https://docs.rs/io-arrays/latest/io_arrays/trait.ReadAt.html#tymethod.read_via_stream_at
