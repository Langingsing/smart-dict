use std::fs::OpenOptions;
use std::{
  io::{self, SeekFrom, Seek, BufReader, BufWriter, BufRead, Write},
  fs::File,
  path::Path,
};

fn open_read_and_write(path: impl AsRef<Path>) -> io::Result<File> {
  OpenOptions::new()
    .read(true)
    .write(true)
    .open(path)
}

/// requires: lines are in strict ascending order
fn remove_lines_sync(mut file: File, mut lines: impl Iterator<Item=usize>) {
  let mut lines = lines.peekable();
  let msg = "can't clone file handle";
  let file1 = file.try_clone().expect(msg);
  let mut file2 = file.try_clone().expect(msg);
  let mut reader = BufReader::new(file);
  let mut writer = BufWriter::new(file1);
  let mut writer_pos = 0;

  for mut line in reader
    .lines()
    .enumerate()
    .filter_map(|(i, line)| {
      match lines.peek() {
        None => None,
        Some(&next_idx) => {
          if next_idx == i {
            lines.next().unwrap();
            None
          } else {
            let line = line.expect("can't read lines");
            Some(line)
          }
        }
      }
    }) {
    line.push_str("\r\n");

    // store reader pos
    let reader_pos = file2.stream_position().expect("can't seek");
    // prepare to write
    file2.seek(SeekFrom::Start(writer_pos)).expect("can't seek");

    writer.write(line.as_bytes()).expect("can't write lines");
    // update writer pos
    writer_pos += line.len() as u64;
    // restore reader pos
    file2.seek(SeekFrom::Start(reader_pos)).expect("can't seek");
  }

  file2.set_len(writer_pos).unwrap();
}

#[cfg(test)]
mod test {
  use std::fs::{File, OpenOptions};
  use std::io::{BufWriter, Write};
  use super::*;

  const PATH: &str = r"D:\temp.txt";

  #[test]
  fn test() {
    remove_lines_sync(
      OpenOptions::new()
        .read(true)
        .write(true)
        .open(PATH)
        .unwrap(),
      [0usize, 3, 4999, 10000, 50000, 50001].into_iter(),
    )
  }

  fn make() {
    let mut file = File::create(PATH).unwrap();
    let mut w = BufWriter::new(file);
    for i in 0..100000 {
      w.write_fmt(format_args!("{i}\r\n")).unwrap();
    }
  }
}
