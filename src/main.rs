use std::path::PathBuf;
use async_std::{
  io::BufReader,
  fs::File,
};
use futures::{future, AsyncBufReadExt, StreamExt};

const DICT_EXT: &str = "dict.yaml";
const SCHEMA: &str = "xkjd6";

#[async_std::main]
async fn main() {
  let custom_dir = get_custom_dir();
  let filename = "xkjd6.extended.dict.yaml";
  let main_dict_path = custom_dir.join(filename);
  let main_dict = File::open(&main_dict_path).await
    .expect(&format!("can't read {:?}", &main_dict_path));

  use regex::Regex;

  let re = Regex::new(r"xkjd6\.[-_\w]+").unwrap();

  BufReader::new(main_dict)
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
    .for_each_concurrent(None, |line| async move {
      println!("{}", line);
    })
    .await;
}

fn get_custom_dir() -> PathBuf {
  use std::{
    env,
    path::Path,
  };

  let args = env::args();
  let mut args = args.skip(1); // skip exe
  let custom_dir = args.next()
    .map_or_else(|| {
      let appdata = env::vars()
        .find(|(key, _)| key == "APPDATA")
        .map(|(_, val)| val)
        .expect("can't read APPDATA from env");
      Path::new(&appdata).join("Rime")
    }, PathBuf::from);

  custom_dir
}
