use std::borrow::{Borrow, Cow};
use std::collections::HashMap;
use std::hash::Hash;
use std::mem;
use crate::trie::Trie;
use crate::types::{Code, Word};

#[derive(Default)]
struct Info<'a> {
  full_code: Code,
  node: Option<&'a Trie>,
}

impl<'a> Info<'a> {
  fn new(full_code: Code) -> Self {
    Self { full_code, ..Default::default() }
  }

  fn from(node: &'a Trie) -> Self {
    Self {
      full_code: node.full_code(),
      node: Some(node),
    }
  }
}

pub struct RevDict<'a> {
  map: HashMap<Word, Info<'a>>,
}

impl<'a> RevDict<'a> {
  pub fn new() -> Self {
    Self { map: HashMap::new() }
  }

  pub fn from_borrowed(pairs: &[(&str, &str)]) -> Self {
    Self::from_iter(pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())))
  }

  pub fn get(&self, word: &str) -> Option<&Code> {
    self.map.get(word).map(|info| &info.full_code)
  }

  pub fn get_mut(&mut self, word: &str) -> Option<&mut Code> {
    self.map.get_mut(word).map(|info| &mut info.full_code)
  }

  pub fn insert(&mut self, word: Word, node: &'a Trie) {
    self.map.insert(word, Info::from(node));
  }

  pub fn insert_if_shorter(&mut self, word: &str, node: &'a Trie) {
    match self.get_mut(word) {
      None => {
        self.insert(word.to_string(), node);
      }
      Some(p_code) => {
        if node.full_code_len() < p_code.len() {
          *p_code = node.full_code()
        }
      }
    }
  }
}

impl RevDict<'_> {
  pub fn shortest(&self, sentence: &str) -> Result<Vec<Code>, String> {
    /*
     * dp[i] = min { dp[j] + self[sentence[j..i]].length } for 0 <= j < i
     * */

    struct State<'a> {
      code: Cow<'a, String>,
      prev: usize,
      sum_len: usize,
    }

    let mut dp = vec![State {
      code: Cow::Owned(String::new()),
      prev: 0,
      sum_len: 0,
    }];

    let char_indices: Vec<_> = sentence.char_indices().collect();
    for (right_char_index, &(_, right_char)) in char_indices.iter().enumerate() {
      let mut code = None;
      let mut prev = 0;
      let mut sum_len = usize::MAX;
      let next_byte_index = char_indices
        .get(right_char_index + 1)
        .map(|pair| pair.0)
        .unwrap_or(sentence.len());
      for left_char_index in 0..=right_char_index {
        let left_byte_index = char_indices[left_char_index].0;
        let word = &sentence[left_byte_index..next_byte_index];

        if let Some(rev_code) = self.get(word) {
          let prev_len = dp[left_char_index].sum_len;
          let new_len = prev_len + rev_code.len();
          if new_len < sum_len {
            sum_len = new_len;
            prev = left_char_index;
            code = Some(rev_code);
          }
        }
      }
      if let Some(code) = code {
        dp.push(State { code: Cow::Borrowed(code), prev, sum_len });
      } else {
        return Err(format!("can't generate the sentence from the dictionary, see '{right_char}' at {right_char_index}"));
      }
    }

    // collect
    let mut codes = vec![];
    let mut node = dp.last().unwrap();
    loop {
      codes.push(node.code.to_string());
      if node.prev == 0 {
        break;
      }
      node = &dp[node.prev];
    }
    codes.reverse();
    Ok(codes)
  }
}

impl FromIterator<(Word, Code)> for RevDict<'_> {
  fn from_iter<T: IntoIterator<Item=(Word, Code)>>(iter: T) -> Self {
    Self { map: HashMap::from_iter(iter.into_iter().map(|(word, code)| (word, Info::new(code)))) }
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn test_shortest() {
    let dict = RevDict::from_borrowed(&[
      ("你", "n "),
      ("好", "h "),
      ("吗", "ms "),
      ("你好", "nau"),
      ("好吗", "hzms "),
    ]);
    let result = dict.shortest("你好吗");
    let ret = result.unwrap();
    assert_eq!(2, ret.len());
    assert_eq!("nau", ret[0]);
    assert_eq!("ms ", ret[1]);
  }
}
