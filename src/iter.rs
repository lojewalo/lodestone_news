use scraper::{Html, Selector, Node};
use ego_tree::iter::{Traverse, Edge};

#[derive(Debug, Clone)]
struct LineBreakText<'a> {
  inner: Traverse<'a, Node>,
  separator: &'a str,
  skipping: bool
}

impl<'a> LineBreakText<'a> {
  fn new(inner: Traverse<'a, Node>, separator: &'a str) -> Self {
    LineBreakText {
      inner,
      separator,
      skipping: false
    }
  }
}

impl<'a> Iterator for LineBreakText<'a> {
  type Item = &'a str;

  fn next(&mut self) -> Option<Self::Item> {
    for edge in &mut self.inner {
      if let Edge::Open(node) = edge {
        match *node.value() {
          Node::Text(ref text) => return Some(&*text),
          Node::Element(ref e) if e.name() == "br" => {
            if self.skipping {
              continue;
            }
            self.skipping = true;
            return Some(self.separator);
          },
          Node::Element(_) if self.skipping => self.skipping = false,
          _ => {}
        }
      }
    }
    None
  }
}
