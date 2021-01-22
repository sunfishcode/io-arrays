#![cfg_attr(can_vector, feature(can_vector))]
#![cfg_attr(write_all_vectored, feature(write_all_vectored))]

use cap_std::fs::OpenOptions;
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

    let mut in_file = dir.create(in_txt)?;
    write!(in_file, "XYZHello, world!XYZ")?;

    // Test regular file I/O.
    {
        let input = FileReader::file(dir.open(in_txt)?);
        let output = FileWriter::file(dir.create(out_txt)?);
        let meta = input.metadata()?;
        let len = meta.len();
        assert_eq!(len, 19);
        let mut buf = vec![0_u8; 13];
        output.allocate(0, 13)?;
        assert_eq!(output.metadata()?.len(), 13);
        input.read_exact_at(&mut buf, 3)?;
        output.write_all_at(&buf, 3)?;
        let mut s = String::new();
        dir.open(out_txt)?.read_to_string(&mut s)?;
        assert_eq!(s, "\0\0\0Hello, world!");
        dir.remove_file(out_txt)?;
    }

    Ok(())
}

#[cfg(feature = "io-streams")]
#[test]
fn test_streaming_read() -> anyhow::Result<()> {
    let dir = tmpdir();
    let in_txt = "in.txt";

    let mut in_file = dir.create(in_txt)?;
    write!(in_file, "XYZHello, world!")?;

    let input = FileReader::file(dir.open(in_txt)?);
    let mut buf = Vec::new();
    input.read_via_stream(3)?.read_to_end(&mut buf)?;
    assert_eq!(&buf, b"Hello, world!");

    let input = FileReader::file(dir.open(in_txt)?);
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

// Test that writing past the end of a file extends the file with zeros.
#[test]
fn test_write_past_end() -> anyhow::Result<()> {
    let dir = tmpdir();
    let name = "file.txt";
    let editor = FileEditor::file(dir.open_with(
        name,
        OpenOptions::new().create_new(true).read(true).write(true),
    )?);
    let message = b"Greetings, world!";
    editor.write_all_at(message, 8192)?;
    let mut buf = vec![0xa0_u8; 8192 + message.len()];
    editor.read_exact_at(&mut buf, 0)?;

    let mut cmp = vec![0_u8; 8192];
    cmp.extend_from_slice(message);
    assert_eq!(buf, cmp);

    Ok(())
}

// Test that reading past the end of a file fails gracefully.
#[test]
fn test_read_past_end() -> anyhow::Result<()> {
    let dir = tmpdir();
    let name = "file.txt";
    let editor = FileEditor::file(dir.open_with(
        name,
        OpenOptions::new().create_new(true).read(true).write(true),
    )?);
    let message = b"Greetings, world!";
    editor.write_all_at(message, 0)?;
    let mut buf = vec![0xa0_u8; 32];
    assert_eq!(editor.read_at(&mut buf, 8192)?, 0);
    assert_eq!(
        editor.read_exact_at(&mut buf, 8192).unwrap_err().kind(),
        std::io::ErrorKind::UnexpectedEof
    );

    Ok(())
}
