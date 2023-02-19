use std::collections::HashMap;
use std::mem;
use crate::types::{Code, Word};

pub fn shortest<'a>(sentence: &String, rev_dict: &'a HashMap<Word, Code>) -> Result<Vec<&'a Code>, String> {
  /*
   * dp[i] = min { dp[j] + rev_dict[sentence[j..i]].length } for 0 <= j < i
   * */

  struct State<'a> {
    code: &'a String,
    prev: usize,
    sum_len: usize,
  }

  let mut dp = vec![State {
    code: unsafe { mem::transmute(&String::new()) },
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

      if let Some(rev_code) = rev_dict.get(word) {
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
      dp.push(State { code, prev, sum_len });
    } else {
      return Err(format!("can't generate the sentence from the dictionary, see '{right_char}' at {right_char_index}"));
    }
  }

  // collect
  let mut codes = vec![];
  let mut node = dp.last().unwrap();
  loop {
    codes.push(node.code);
    if node.prev == 0 {
      break;
    }
    node = &dp[node.prev];
  }
  codes.reverse();
  Ok(codes)
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn test() {
    let entries = [
      ("你", "n "),
      ("好", "h "),
      ("吗", "ms "),
      ("你好", "nau"),
      ("好吗", "hzms "),
    ];
    let dict = entries.iter().map(|(s1, s2)| (s1.to_string(), s2.to_string())).collect();
    let result = shortest(&"你好吗".into(), &dict);
    let ret = result.unwrap();
    assert_eq!(2, ret.len());
    assert_eq!("nau", ret[0]);
    assert_eq!("ms ", ret[1]);
  }
}
