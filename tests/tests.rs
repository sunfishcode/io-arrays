#![cfg_attr(can_vector, feature(can_vector))]
#![cfg_attr(write_all_vectored, feature(write_all_vectored))]

use cap_tempfile::{tempdir, TempDir};
use io_files::{FileEditor, FileReader, FileWriter, MinimalFile, ReadAt, WriteAt};
use std::io::{Read, Write};

#[allow(unused)]
fn tmpdir() -> TempDir {
    unsafe { tempdir().expect("expected to be able to create a temporary directory") }
}

#[test]
fn test_small_copy() -> anyhow::Result<()> {
    let dir = tmpdir();
    let in_txt = "in.txt";
    let out_txt = "out.txt";

    let mut in_file = dir.create(&in_txt)?;
    write!(in_file, "XYZHello, world!XYZ")?;

    // Test regular file I/O.
    {
        let input = FileReader::file(dir.open(&in_txt)?);
        let output = FileWriter::file(dir.create(&out_txt)?);
        let meta = input.metadata()?;
        let len = meta.len();
        assert_eq!(len, 19);
        let mut buf = vec![0_u8; 13];
        output.allocate(0, 13)?;
        assert_eq!(output.metadata()?.len(), 13);
        input.read_exact_at(&mut buf, 3)?;
        output.write_all_at(&buf, 3)?;
        let mut s = String::new();
        dir.open(&out_txt)?.read_to_string(&mut s)?;
        assert_eq!(s, "\0\0\0Hello, world!");
        dir.remove_file(&out_txt)?;
    }

    Ok(())
}

#[cfg(feature = "io-streams")]
#[test]
fn test_streaming_read() -> anyhow::Result<()> {
    let dir = tmpdir();
    let in_txt = "in.txt";

    let mut in_file = dir.create(&in_txt)?;
    write!(in_file, "XYZHello, world!")?;

    let input = FileReader::file(dir.open(&in_txt)?);
    let mut buf = Vec::new();
    input.read_via_stream(3)?.read_to_end(&mut buf)?;
    assert_eq!(&buf, b"Hello, world!");

    let input = FileReader::file(dir.open(&in_txt)?);
    let mut buf = Vec::new();
    input.read_via_stream(3)?.read_to_end(&mut buf)?;
    assert_eq!(&buf, b"Hello, world!");

    Ok(())
}

#[test]
fn test_bytes() -> anyhow::Result<()> {
    let reader = FileReader::bytes(b"abcdefghij")?;
    let mut buf = vec![0_u8; 4];
    reader.read_exact_at(&mut buf, 3)?;
    assert_eq!(buf, b"defg");
    Ok(())
}

#[test]
fn test_anonymous() -> anyhow::Result<()> {
    let editor = FileEditor::anonymous()?;
    editor.write_all_at(b"0123456789", 5)?;
    let mut buf = vec![0_u8; 4];
    editor.read_exact_at(&mut buf, 8)?;
    assert_eq!(buf, b"3456");
    Ok(())
}
