use std::collections::hash_map::{Keys, Values, ValuesMut};
use std::collections::HashMap;
use std::fmt::Write;
use std::ptr::NonNull;

type Word = String;
type Code = String;

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
  fn poll(&self, chars: &mut impl Iterator<Item=char>) {
    let mut char_pairs = chars.zip(self.code.chars());
    // char_pairs.skip_while()
    char_pairs.all(|(c1, c2)| c1 == c2);
  }

  pub fn insert(&mut self, mut node: Trie<'a>) {
    let descendant = self.try_best_to_match(&mut node.code);
    descendant.set_link(node);
  }

  fn try_best_to_match(&mut self, code: &mut String) -> &mut Trie<'_> {
    let mut chars = code.chars().peekable();
    let mut node_ptr = NonNull::from(self);

    while let Some(&ch) = chars.peek() {
      let node = unsafe { node_ptr.as_mut() };

      let option = node
        .children_mut()
        .find(|child| child.code.starts_with(ch));

      if let Some(child) = option {
        node_ptr = NonNull::from(child);
      } else {
        break;
      }

      node.poll(&mut chars);
    }

    let remained: String = chars.collect();
    code.truncate(0);
    code.write_str(&remained).unwrap();
    code.shrink_to_fit();

    unsafe { node_ptr.as_mut() }
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
