mod fileman;

use std::fmt::{Display, Formatter};
use std::path::PathBuf;
use async_std::{
  io::{BufReader, BufWriter, WriteExt},
  fs::File,
};
use futures::{future, AsyncBufReadExt, StreamExt};
use lazy_static::lazy_static;

const DICT_EXT: &str = "dict.yaml";
const SCHEMA: &str = "xkjd6";

lazy_static! {
  static ref CUSTOM_DIR: PathBuf = get_custom_dir();
}

struct Data {
  name: String,
  size: usize,
  word_len: usize,
  code_len: usize,
}

impl Display for Data {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{n},{wl},{cl},{sum},{wr:.2},{cr:.2},{sr:.2}",
           n = self.name,
           wl = self.word_len,
           cl = self.code_len,
           sum = self.sum(),
           wr = self.word_ratio() * 100.0,
           cr = self.code_ratio() * 100.0,
           sr = self.sum_ratio() * 100.0
    )
  }
}

impl Data {
  fn sum(&self) -> usize {
    self.word_len + self.code_len
  }

  fn word_ratio(&self) -> f64 {
    self.word_len as f64 / self.size as f64
  }

  fn code_ratio(&self) -> f64 {
    self.code_len as f64 / self.size as f64
  }

  fn sum_ratio(&self) -> f64 {
    self.sum() as f64 / self.size as f64
  }
}

async fn statistic(dict_name: &str) -> Data {
  let filename = format!("{dict_name}.{DICT_EXT}");
  let path = CUSTOM_DIR.join(&filename);
  let data = BufReader::new(File::open(&path).await.unwrap())
    .lines()
    .filter_map(|line| async {
      let line = line.expect(&format!("can't read from {}", &filename));
      line.split_once('\t').map(|(word, code)| {
        (word.len(), code.len())
      })
    })
    .fold(Data {
      name: dict_name.to_owned(),
      size: async_std::fs::metadata(path).await.unwrap().len() as usize,
      word_len: 0,
      code_len: 0,
    }, |mut acc, item| async move {
      acc.word_len += item.0;
      acc.code_len += item.1;
      acc
    })
    .await;
  data
}

#[async_std::main]
async fn main() {
  let filename = "xkjd6.extended.dict.yaml";
  let main_dict_path = CUSTOM_DIR.join(filename);
  let main_dict = File::open(&main_dict_path).await
    .expect(&format!("can't read {:?}", &main_dict_path));

  use regex::Regex;

  let re = Regex::new(r"xkjd6\.[-_\w]+").unwrap();

  let async_iter = BufReader::new(main_dict)
    .lines()
    .map(|line| line.expect("can't read lines "))
    .filter(|line| future::ready(!line.is_empty() && !line.starts_with("#")))
    .skip_while(|line| future::ready(!line.starts_with("import_tables")))
    .skip(1) // skip "import_tables:"
    .map(|line| {
      let m = re.find(&line).unwrap();
      let start = m.start();
      let end = m.end();
      String::from(&line[start..end])
    })
    .map(|dict_name| async move {
      statistic(&dict_name).await
    })
    .collect::<Vec<_>>()
    .await;
  let mut result: Vec<Data> = future::join_all(async_iter).await;
  result.sort_by(|a, b| {
    b.sum_ratio().partial_cmp(&a.sum_ratio()).unwrap()
  });
  let out = File::create("data.csv").await.unwrap();
  let mut writer = BufWriter::new(out);
  writeln!(writer, "name,word len,code len,sum,word per,code per,sum per").await.unwrap();

  for x in result {
    writeln!(writer, "{x}").await.unwrap();
  }
}

fn get_custom_dir() -> PathBuf {
  use std::{
    env,
    path::Path,
  };

  let args = env::args();
  let mut args = args.skip(1); // skip exe
  args.next()
    .map_or_else(|| {
      let appdata = env::vars()
        .find(|(key, _)| key == "APPDATA")
        .map(|(_, val)| val)
        .expect("can't read APPDATA from env");
      Path::new(&appdata).join("Rime")
    }, PathBuf::from)
}
