use std::ops::Range;
use std::collections::HashMap;
use crate::trie::Trie;
use crate::types::{Code, Word};

struct Info<'a> {
  full_code: Code,
  node: &'a Trie,
}

impl<'a> Info<'a> {
  fn from(node: &'a Trie) -> Self {
    Self {
      full_code: node.full_code(),
      node,
    }
  }
}

pub struct RevDict<'a> {
  map: HashMap<Word, Info<'a>>,
  trie: &'a Trie,
}

impl<'a> RevDict<'a> {
  pub fn new(trie: &'a Trie) -> Self {
    Self { map: HashMap::new(), trie }
  }

  fn get(&self, word: &str) -> Option<&Info<'a>> {
    self.map.get(word)
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
      code: String,
      prev: usize,
      sum_len: usize,
      node: &'a Trie,
      word_range: Range<usize>,
    }

    let mut dp = vec![State {
      code: "".to_string(),
      prev: 0,
      sum_len: 0,
      node: self.trie,
      word_range: Default::default(),
    }];

    let char_indices: Vec<_> = sentence.char_indices().collect();
    for (right_char_index, &(_, right_char)) in char_indices.iter().enumerate() {
      let mut code = String::new();
      let mut prev = 0;
      let mut sum_len = usize::MAX;
      let mut node_option = None;
      let mut word_range = Default::default();
      let next_byte_index = char_indices
        .get(right_char_index + 1)
        .map(|pair| pair.0)
        .unwrap_or(sentence.len());
      for left_char_index in 0..=right_char_index {
        let left_byte_index = char_indices[left_char_index].0;
        word_range = left_byte_index..next_byte_index;
        let word = &sentence[word_range.clone()];

        if let Some(Info { full_code: rev_code, node }) = self.get(word) {
          let prev_state = &dp[left_char_index];
          let prefix_blank = {
            let prev_node = prev_state.node;
            let is_prev_candidate = {
              let prev_word = &sentence[prev_state.word_range.clone()];
              let mut prev_candidates = prev_node.candidates();
              if let Some(first_candidate) = prev_candidates.next() {
                first_candidate == prev_word && prev_candidates.next().is_some()
              } else {
                false
              }
            };
            is_prev_candidate
              && prev_node.children().any(|child| rev_code.starts_with(child.code()))
          };

          let prev_len = prev_state.sum_len;
          let new_len = prev_len + rev_code.len() + if prefix_blank { 1 } else { 0 };
          if new_len < sum_len {
            sum_len = new_len;
            prev = left_char_index;
            code = format!("{}{rev_code}", if prefix_blank { " " } else { "" });
            node_option = Some(node);
          }
        }
      }
      if let Some(node) = node_option {
        dp.push(State { code, prev, sum_len, node, word_range });
      } else {
        return Err(format!("can't generate the sentence from the dictionary, see '{right_char}' at {right_char_index}"));
      }
    }

    // collect
    let mut codes = vec![];
    let mut state = dp.last().unwrap();
    if state.node.words().len() > 1 || !state.node.is_leaf() {
      codes.push(" ".to_string());
    }
    loop {
      codes.push(state.code.clone());
      if state.prev == 0 {
        break;
      }
      state = &dp[state.prev];
    }
    codes.reverse();
    Ok(codes)
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn test_shortest() {
    let mut trie = Trie::new();
    let mut path = home::home_dir().unwrap();

    path.push(r"AppData\Roaming\Rime");
    path.push("xkjd6.cizu.dict.yaml");
    trie.load_xkjd_dict(&path).unwrap();

    path.pop();
    path.push("xkjd6.danzi.dict.yaml");
    trie.load_xkjd_dict(&path).unwrap();

    path.pop();
    path.push("xkjd6.wxw.dict.yaml");
    trie.load_xkjd_dict(&path).unwrap();

    assert_eq!("我爱读书", trie.eval("wlxhdjej "));

    trie.check_links().unwrap();

    let dict = trie.rev_dict();
    let ret = dict.shortest("你好吗").unwrap();
    assert_eq!(vec!["nau", "ms", " "], ret);
  }
}
