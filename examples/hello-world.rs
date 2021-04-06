use io_arrays::{ArrayReader, ReadAt};
use io_streams::StreamWriter;
use std::io::copy;

fn main() -> anyhow::Result<()> {
    let reader = ArrayReader::bytes(b"123hello world456")?;

    // Start at offset 3 and read 11 bytes from input.
    let mut buf = vec![0_u8; 11];
    reader.read_exact_at(&mut buf, 3)?;
    // The buffer can be used directly.
    // println!("{}", std::str::from_utf8(&buf)?);

    // Or it can be transformed into a stream.
    let mut stream = ArrayReader::bytes(&buf)?.read_via_stream_at(0)?;
    let mut stdout = StreamWriter::stdout()?;
    copy(&mut stream, &mut stdout)?;
    Ok(())
}
