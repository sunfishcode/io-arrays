#![cfg_attr(can_vector, feature(can_vector))]
#![cfg_attr(write_all_vectored, feature(write_all_vectored))]

use io_files::{FileReader, FileWriter, MinimalFile, ReadAt, WriteAt};
use std::{
    fs::{remove_file, File},
    io::{Read, Write},
};
use tempfile::{tempdir, TempDir};

#[allow(unused)]
fn tmpdir() -> TempDir {
    tempdir().expect("expected to be able to create a temporary directory")
}

#[test]
fn test_small_copy() -> anyhow::Result<()> {
    let dir = tmpdir();
    let in_txt = dir.path().join("in.txt");
    let out_txt = dir.path().join("out.txt");

    let mut in_file = File::create(&in_txt)?;
    write!(in_file, "XYZHello, world!XYZ")?;

    // Test regular file I/O.
    {
        let input = FileReader::file(File::open(&in_txt)?);
        let output = FileWriter::file(File::create(&out_txt)?);
        let meta = input.metadata()?;
        let len = meta.len();
        assert_eq!(len, 19);
        let mut buf = vec![0u8; 13];
        output.allocate(0, 13)?;
        assert_eq!(output.metadata()?.len(), 13);
        input.read_exact_at(&mut buf, 3)?;
        output.write_all_at(&buf, 3)?;
        let mut s = String::new();
        File::open(&out_txt)?.read_to_string(&mut s)?;
        assert_eq!(s, "\0\0\0Hello, world!");
        remove_file(&out_txt)?;
    }

    Ok(())
}

#[cfg(feature = "io-streams")]
#[test]
fn test_streaming_read() -> anyhow::Result<()> {
    let dir = tmpdir();
    let in_txt = dir.path().join("in.txt");

    let mut in_file = File::create(&in_txt)?;
    write!(in_file, "XYZHello, world!")?;

    let input = FileReader::file(File::open(&in_txt)?);
    let mut buf = Vec::new();
    input.read_via_stream(3)?.read_to_end(&mut buf)?;
    assert_eq!(&buf, b"Hello, world!");

    let input = FileReader::file(File::open(&in_txt)?);
    let mut buf = Vec::new();
    input.read_via_stream(3)?.read_to_end(&mut buf)?;
    assert_eq!(&buf, b"Hello, world!");

    Ok(())
}
