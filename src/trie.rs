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

impl Trie {
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

  fn children_mut(&mut self) -> ValuesMut<'_, Code, Self> {
    self.links.values_mut()
  }

  pub fn edges(&self) -> Keys<'_, Code, Self> {
    self.links.keys()
  }

  pub fn nodes(&self) -> Nodes<'_> {
    Nodes::new(self)
  }

  pub fn child(&self, child_code: &str) -> Option<&Self> {
    self.links.get(child_code)
  }

  pub fn child_mut(&mut self, child_code: &str) -> Option<&mut Self> {
    self.links.get_mut(child_code)
  }

  fn set_half_parent_nonnull(&mut self, p_parent: NonNull<Self>) {
    self.parent.replace(p_parent);
  }

  fn set_half_parent(&mut self, parent: &Self) {
    self.set_half_parent_nonnull(NonNull::from(parent));
  }

  fn set_half_link(&mut self, child: Self) -> Option<Self> {
    self.links.insert(child.code.clone(), child)
  }

  unsafe fn set_half_link_and_borrow<'a>(&mut self, child: Self) -> &'a mut Self {
    let code = child.code.clone();
    self.set_half_link(child);
    mem::transmute(self.child_mut(&code).unwrap())
  }

  fn del_half_link(&mut self, key: &Code) -> Option<Self> {
    self.links.remove(key)
  }

  fn set_link(&mut self, child: Self) -> &mut Self {
    let child = unsafe { self.set_half_link_and_borrow(child) };
    child.set_half_parent(self);
    child
  }
}

impl Trie {
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
      let mut this = parent.del_half_link(&self.code).unwrap();

      this.code.truncate(new_len);
      this.code.shrink_to_fit();

      parent.set_half_link_and_borrow(this)
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
        let new_node = Self {
          code: child_code,
          words: mem::replace(&mut node.words, vec![word]),
          links: mem::take(&mut node.links),
          parent: None,
        };
        let new_node = node.set_link(new_node);

        let p_new_node = unsafe { NonNull::new_unchecked(new_node) };
        for child in new_node.children_mut() {
          child.set_half_parent_nonnull(p_new_node);
        }
      }
    } else {
      let remained_code = code.into_remained();

      if matched == node.code.len() {
        let p_node = unsafe { NonNull::new_unchecked(node) };
        node.set_half_link(Self {
          code: remained_code,
          words: vec![word],
          parent: Some(p_node),
          ..Default::default()
        });
      } else {
        // regard node as the new parent and construct two new children
        let child_code = node.code[matched..].to_string();
        let node = unsafe { node.shrink_code(matched) };
        let spawn_child = Self {
          code: child_code,
          words: mem::take(&mut node.words),
          links: mem::take(&mut node.links),
          parent: None,
        };
        let spawn_child = node.set_link(spawn_child);

        let p_spawn_child = unsafe { NonNull::new_unchecked(spawn_child) };
        for grandchild in spawn_child.children_mut() {
          grandchild.set_half_parent_nonnull(p_spawn_child);
        }

        let new_child = Self {
          code: remained_code,
          words: vec![word],
          parent: None,
          ..Default::default()
        };
        node.set_link(new_child);
      }
    }
  }

  fn try_best_to_match(&mut self, code: &mut CodeCursor) -> (&mut Self, usize) {
    let mut matched;
    let mut node_ptr = NonNull::from(self);

    loop {
      let node = unsafe { node_ptr.as_ref() };
      matched = node.poll(code);

      if code.is_empty() || matched < node.code.len() {
        break;
      }

      let ch = code[0] as char;
      let option = node.child(&String::from(ch))
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

impl Trie {
  pub fn build_rev_dict(&self) -> HashMap<Word, &Code> {
    let mut rev_dict = HashMap::new();
    for node in self.nodes() {
      let Trie { code, words, .. } = node;
      for word in words {
        match rev_dict.get_mut(word) {
          None => {
            rev_dict.insert(word.clone(), code);
          }
          Some(p_code) => {
            if code.len() < p_code.len() {
              *p_code = code
            }
          }
        }
      }
    }
    rev_dict
  }
}

#[cfg(test)]
impl Trie {
  fn check_links(&self) -> Result<(), &Self> {
    for child in self.children() {
      if child.parent().unwrap() as *const _ != self as *const _ {
        return Err(self);
      }
      child.check_links()?;
    }
    Ok(())
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
  use std::collections::HashSet;
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

    let trie = root.child("n").unwrap();
    assert_eq!("n", trie.code);
    assert_eq!(vec!["你".to_string()], trie.words);
    assert_eq!(&root as *const _, trie.parent().unwrap() as *const _);
    assert_eq!(1, trie.children().count());

    let child = trie.child("i").unwrap();
    assert_eq!("i", child.code);
    assert_eq!(vec!["你们".to_string()], child.words);
    assert_eq!(trie as *const _, child.parent().unwrap() as *const _);
    assert_eq!(1, child.children().count());

    let descendant = &child.links["a"];
    assert_eq!("a", descendant.code);
    assert_eq!(vec!["哪里".to_string()], descendant.words);
    assert_eq!(child as *const _, descendant.parent().unwrap() as *const _);
    assert_eq!(0, descendant.children().count());

    assert!(root.check_links().is_ok());
  }

  #[test]
  fn test_insert_with_extraction() {
    let mut root = Trie::default();
    root.insert("ni".to_string(), "你们".to_string());
    root.insert("na".to_string(), "能力".to_string());
    assert_eq!("", root.code);
    assert!(root.words.is_empty());
    assert_eq!(None, root.parent);
    assert_eq!(1, root.children().count());

    let trie = root.child("n").unwrap();
    assert_eq!("n", trie.code);
    assert!(trie.words.is_empty());
    assert_eq!(&root as *const _, trie.parent().unwrap() as *const _);
    assert_eq!(2, trie.children().count());

    let child1 = trie.child("i").unwrap();
    assert_eq!("i", child1.code);
    assert_eq!(vec!["你们".to_string()], child1.words);
    assert_eq!(trie as *const _, child1.parent().unwrap() as *const _);
    assert_eq!(0, child1.children().count());

    let child2 = trie.child("a").unwrap();
    assert_eq!("a", child2.code);
    assert_eq!(vec!["能力".to_string()], child2.words);
    assert_eq!(trie as *const _, child2.parent().unwrap() as *const _);
    assert_eq!(0, child2.children().count());

    assert!(root.check_links().is_ok());
  }

  #[test]
  fn test_nodes_iter() {
    let mut root = Trie::default();
    root.insert("m".to_string(), "没".to_string());
    root.insert("ni".to_string(), "你们".to_string());
    root.insert("nia".to_string(), "哪里".to_string());
    root.insert("n".to_string(), "你".to_string());

    let codes: Vec<_> = root.nodes().map(|node| &node.code).collect();
    assert_eq!(5, codes.len());
    assert_eq!(
      HashSet::<_>::from_iter(["", "m", "n", "i", "a"].map(|s| s.to_string())),
      HashSet::from_iter(codes.iter().map(|s| s.to_string()))
    );
  }
}
