use scraper::{Node, ElementRef};
use ego_tree::iter::{Traverse, Edge};

use std::borrow::Cow;

#[derive(Debug, Clone)]
pub struct NewsText<'a> {
  inner: Traverse<'a, Node>,
  separator: &'a str,
  skipping: bool
}

impl<'a> NewsText<'a> {
  pub fn new(inner: Traverse<'a, Node>, separator: &'a str) -> Self {
    NewsText {
      inner,
      separator,
      skipping: false
    }
  }
}

impl<'a> Iterator for NewsText<'a> {
  type Item = Cow<'a, str>;

  fn next(&mut self) -> Option<Self::Item> {
    for edge in &mut self.inner {
      if let Edge::Open(node) = edge {
        match *node.value() {
          Node::Text(ref text) => {
            if node.ancestors().any(|a| match *a.value() {
              Node::Element(ref e) if e.name() == "a" => true,
              _ => false
            }) {
              continue;
            }
            return Some(Cow::Borrowed(&*text))
          },
          Node::Element(ref e) if e.name() == "a" => {
            if let Some(href) = e.attr("href") {
              let text: String = ElementRef::wrap(node).unwrap().text().collect();
              return Some(Cow::Owned(format!("[{}]({})", text, href)));
            }
          }
          Node::Element(ref e) if e.name() == "br" => {
            if self.skipping {
              continue;
            }
            self.skipping = true;
            return Some(Cow::Borrowed(self.separator));
          },
          _ => {
            if self.skipping {
              self.skipping = false;
            }
          }
        }
      }
    }
    None
  }
}
