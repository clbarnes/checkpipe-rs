# checkpipe

A rust library for computing things about bytes as they flow through pipelines.

The motivating use case is performing a checksum on all bytes passing through a reader or writer.

## Example

```rust
use checkpipe::{Check, Checker};
use std::io::{Read, Write};
use std::fs::{File, remove_file};

const DATA: [u8; 13] = *b"Hello, world!";
const PATH: &'static str = "foo.txt";

pub fn write_checksummed() -> std::io::Result<()> {
    let outfile = File::create(PATH)?;
    let mut outpipe = Checker::new_default_hasher(outfile);

    outpipe.write_all(DATA.as_slice())?;
    let checksum = outpipe.output();
    outpipe.write(&mut checksum.to_le_bytes()[..])?;
    Ok(())
}

pub fn read_checksummed() -> std::io::Result<()> {
    let infile = File::open(PATH)?;
    let mut inpipe = Checker::new_default_hasher(infile);

    let mut buf = [0u8; DATA.len()];
    inpipe.read(&mut buf)?;
    let checksum = inpipe.output();

    let mut checksum_buf = [0u8; 8];
    inpipe.read(&mut checksum_buf)?;
    assert_eq!(checksum, u64::from_le_bytes(checksum_buf));
    Ok(())
}

pub fn main() -> std::io::Result<()> {
    write_checksummed()?;
    read_checksummed()?;
    remove_file(PATH)
}
```

## Implementing your own checks

Implement the `Check` trait on a struct, which provides methods for taking a chunk of bytes,
and eventually returning the result of its computation.
Then wrap that struct and some struct through which bytes pass in a `Checker`.

`Check` is already implemented for all types implementing `Hasher`,
and there are some convenience methods in place if you want to use rust's default hasher.
If the wrapped type in `Checker` is `Read` or `Write`, so too will the `Checker`.
