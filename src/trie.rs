use std::collections::hash_map::{Keys, Values, ValuesMut};
use std::collections::HashMap;
use std::io::Cursor;
use std::mem;
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
pub struct Trie {
  code: Code,
  words: Vec<Word>,
  parent: Option<NonNull<Self>>,
  links: HashMap<Code, Self>,
}

impl<'a> Trie {
  pub fn new(code: Code) -> Self {
    Self {
      code,
      ..Default::default()
    }
  }

  pub fn parent(&self) -> Option<&Self> {
    self.parent.map(|p| unsafe { p.as_ref() })
  }

  fn parent_mut(&self) -> Option<&mut Self> {
    self.parent.map(|mut p| unsafe { p.as_mut() })
  }

  pub fn children(&self) -> Values<'_, Code, Self> {
    self.links.values()
  }

  pub fn edges(&self) -> Keys<'_, Code, Self> {
    self.links.keys()
  }

  pub fn nodes(&self) -> Nodes<'_> {
    Nodes::new(self)
  }

  fn set_link(&mut self, child: Self) -> Option<Self> {
    self.links.insert(child.code.clone(), child)
  }

  fn del_link(&mut self, key: &Code) -> Option<Self> {
    self.links.remove(key)
  }

  fn children_mut(&mut self) -> ValuesMut<'_, Code, Self> {
    self.links.values_mut()
  }
}

impl<'a> Trie {
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

  /// SAFETY:
  /// This function returns a temporary reference to a trie node owned by parent.links,
  /// use it only when parent.links won't be moved or reallocated.
  /// `self as * const _ == self.shrink_code(new_len) as * const _` is not guaranteed to be true.
  unsafe fn shrink_code(&mut self, new_len: usize) -> &mut Self {
    debug_assert!(new_len < self.code.len());

    if let Some(parent) = self.parent_mut() {
      let mut this = parent.del_link(&self.code).unwrap();

      this.code.truncate(new_len);
      this.code.shrink_to_fit();
      let new_code = this.code.clone();

      parent.set_link(this);

      mem::transmute(parent.links.get_mut(&new_code).unwrap())
    } else {
      self.code.truncate(new_len);
      self.code.shrink_to_fit();
      self
    }
  }

  pub fn insert(&mut self, code: Code, word: Word) {
    let mut code = CodeCursor::new(code);
    let (node, matched) = self.try_best_to_match(&mut code);
    if code.is_empty() {
      if matched == node.code.len() {
        node.words.push(word)
      } else {
        // regard node as the new parent and construct a new child
        let child_code = node.code[matched..].to_string();
        let node = unsafe { node.shrink_code(matched) };
        let mut new_node = Self {
          code: child_code.clone(),
          words: mem::replace(&mut node.words, vec![word]),
          links: mem::take(&mut node.links),
          parent: None,
        };
        node.set_link(new_node);
        let new_node: &mut Self = unsafe { mem::transmute(node.links.get_mut(&child_code).unwrap()) };
        new_node.parent = Some(NonNull::from(node));

        let p_new_node = unsafe { NonNull::new_unchecked(new_node) };
        for child in new_node.children_mut() {
          child.parent.replace(p_new_node);
        }
      }
    } else {
      let code = code.into_remained();
      if matched == node.code.len() {
        let p_node = unsafe { NonNull::new_unchecked(node) };
        node.set_link(Self {
          code,
          words: vec![word],
          parent: Some(p_node),
          ..Default::default()
        });
      } else {
        todo!()
      }
    }
  }

  fn try_best_to_match(&mut self, code: &mut CodeCursor) -> (&'a mut Self, usize) {
    let mut matched = 0;
    let mut node_ptr = NonNull::from(self);

    loop {
      let node = unsafe { node_ptr.as_ref() };
      matched = node.poll(code);

      if code.is_empty() || matched < node.code.len() {
        break;
      }

      let ch = code[0] as char;
      let option = node.links.get(&String::from(ch))
        .or(node
          .children()
          .find(|child| child.code.starts_with(ch)));

      if let Some(child) = option {
        node_ptr = NonNull::from(child);
      } else {
        break;
      }
    }

    unsafe { (node_ptr.as_mut(), matched) }
  }

  fn find_a_child_starts_with(&self, ch: char) -> Option<&'_ Self> {
    self.children().find(|child| child.code.starts_with(ch))
  }
}

pub struct Nodes<'a> {
  stack: Vec<&'a Trie>,
}

impl<'a> Nodes<'a> {
  pub fn new(root: &'a Trie) -> Self {
    Self {
      stack: vec![root]
    }
  }
}

impl<'a> Iterator for Nodes<'a> {
  type Item = &'a Trie;

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
    assert_eq!("ao", code.into_remained());
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
    assert_eq!("e", code.into_remained());
  }

  #[test]
  fn test_insert_split_without_grandparent() {
    let mut trie = Trie {
      code: "ni".to_string(),
      words: vec!["你们".to_string()],
      ..Default::default()
    };
    trie.insert("n".to_string(), "你".to_string());
    assert_eq!("n", trie.code);
    assert_eq!(vec!["你".to_string()], trie.words);
    assert_eq!(None, trie.parent);
    assert_eq!(1, trie.children().count());
    let child = &trie.links["i"];
    assert_eq!("i", child.code);
    assert_eq!(vec!["你们".to_string()], child.words);
    assert_eq!(&trie as *const _, child.parent().unwrap() as *const _);
    assert!(child.links.is_empty());
  }

  #[test]
  fn test_insert_split_with_grandparent() {
    let mut root = Trie::default();
    root.insert("m".to_string(), "没".to_string());
    root.insert("ni".to_string(), "你们".to_string());
    root.insert("nia".to_string(), "哪里".to_string());
    root.insert("n".to_string(), "你".to_string());
    assert_eq!("", root.code);
    assert!(root.words.is_empty());
    assert_eq!(None, root.parent);
    assert_eq!(2, root.children().count());

    let trie = &root.links["n"];
    assert_eq!("n", trie.code);
    assert_eq!(vec!["你".to_string()], trie.words);
    assert_eq!(&root as *const _, trie.parent().unwrap() as *const _);
    assert_eq!(1, trie.children().count());

    let child = &trie.links["i"];
    assert_eq!("i", child.code);
    assert_eq!(vec!["你们".to_string()], child.words);
    assert_eq!(trie as *const _, child.parent().unwrap() as *const _);
    assert_eq!(1, child.children().count());

    let descendant = &child.links["a"];
    assert_eq!("a", descendant.code);
    assert_eq!(vec!["哪里".to_string()], descendant.words);
    assert_eq!(child as *const _, descendant.parent().unwrap() as *const _);
    assert_eq!(0, descendant.children().count());
  }
}
