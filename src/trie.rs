use std::collections::hash_map::{Keys, Values, ValuesMut};
use std::collections::HashMap;
use std::io::Cursor;
use std::ops::{AddAssign, Index};
use std::ptr::NonNull;
use crate::types::{Code, Word};

struct CodeCursor(Cursor<Code>);

impl CodeCursor {
  pub fn new(code: Code) -> Self {
    Self(Cursor::new(code))
  }

  pub fn position(&self) -> usize {
    self.0.position() as usize
  }

  pub fn get_ref(&self) -> &Code {
    self.0.get_ref()
  }

  pub fn remained_len(&self) -> usize {
    self.get_ref().len() - self.position()
  }

  pub fn is_empty(&self) -> bool {
    self.remained_len() == 0
  }

  pub fn advance_by(&mut self, step: usize) {
    self.0.set_position((self.position() + step) as u64);
  }

  pub fn into_remained(self) -> Code {
    let i = self.position();
    let string = self.0.into_inner();
    string[i..].to_string()
  }
}

impl AddAssign<usize> for CodeCursor {
  fn add_assign(&mut self, rhs: usize) {
    self.advance_by(rhs);
  }
}

impl Index<usize> for CodeCursor {
  type Output = u8;

  fn index(&self, index: usize) -> &Self::Output {
    let pos = self.position();
    &self.get_ref().as_bytes()[pos + index]
  }
}

#[derive(Default)]
pub struct Trie<'a> {
  code: Code,
  words: Vec<Word>,
  parent: Option<&'a Trie<'a>>,
  links: HashMap<Code, Trie<'a>>,
}

impl<'a> Trie<'a> {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn parent(&self) -> Option<&Trie<'_>> {
    self.parent
  }

  pub fn children(&self) -> Values<'_, Code, Trie<'_>> {
    self.links.values()
  }

  pub fn edges(&self) -> Keys<'_, Code, Trie<'_>> {
    self.links.keys()
  }

  pub fn nodes(&self) -> Nodes<'_> {
    Nodes::new(self)
  }

  fn set_link(&mut self, child: Trie<'a>) -> Option<Trie<'_>> {
    self.links.insert(child.code.clone(), child)
  }

  fn children_mut(&mut self) -> ValuesMut<'_, Code, Trie<'a>> {
    self.links.values_mut()
  }
}

impl<'a> Trie<'a> {
  fn poll(&self, code: &mut CodeCursor) -> usize {
    let mut matched = 0;
    while let Some(&ch) = self.code.as_bytes().get(matched) {
      if code.is_empty() || ch != code[0] {
        break;
      }
      matched += 1;
      *code += 1;
    }
    matched
  }

  pub fn insert(&mut self, mut node: Trie<'a>) {
    todo!();
  }

  fn try_best_to_match(&mut self, code: &mut CodeCursor) -> (&'a mut Trie<'a>, usize) {
    let mut node_ptr = NonNull::from(self);
    let matched = 0;

    while !code.is_empty() {
      let ch = code[0] as char;
      let node = unsafe { node_ptr.as_ref() };
      let option = node
        .children()
        .find(|child| child.code.starts_with(ch));

      if let Some(child) = option {
        node_ptr = NonNull::from(child);
      } else {
        break;
      }

      node.poll(code);
    }

    unsafe { (node_ptr.as_mut(), matched) }
  }

  fn find_a_child_starts_with(&self, ch: char) -> Option<&'_ Trie<'_>> {
    self.children().find(|child| child.code.starts_with(ch))
  }
}

pub struct Nodes<'a> {
  stack: Vec<&'a Trie<'a>>,
}

impl<'a> Nodes<'a> {
  pub fn new(root: &'a Trie) -> Self {
    Self {
      stack: vec![root]
    }
  }
}

impl<'a> Iterator for Nodes<'a> {
  type Item = &'a Trie<'a>;

  fn next(&mut self) -> Option<Self::Item> {
    self.stack
      .pop()
      .map(|node| {
        self.stack.extend(node.children());
        node
      })
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn test_poll_short_code() {
    let trie = Trie {
      code: "ni".to_string(),
      ..Default::default()
    };
    let mut code = CodeCursor::new("niao".to_string());
    let matched = trie.poll(&mut code);
    assert_eq!(trie.code.len(), matched);
    assert_eq!("ao".to_string(), code.into_remained());
  }

  #[test]
  fn test_poll_long_code() {
    let trie = Trie {
      code: "niao".to_string(),
      ..Default::default()
    };
    let mut code = CodeCursor::new("ni".to_string());
    let matched = trie.poll(&mut code);
    assert_eq!(2, matched);
    assert!(code.into_remained().is_empty());
  }

  #[test]
  fn test_poll_mismatched_code() {
    let trie = Trie {
      code: "niao".to_string(),
      ..Default::default()
    };
    let mut code = CodeCursor::new("nie".to_string());
    let matched = trie.poll(&mut code);
    assert_eq!(2, matched);
    assert_eq!("e".to_string(), code.into_remained());
  }
}
