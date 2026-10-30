//! A screenshot gallery: data, a type, a list, and a wrapping section.
//! Prints the expanded tree's outline, then its HTML.
//!
//! ```text
//! cargo run --example gallery
//! ```

use libtypedmd::{parse_with, ParseOptions};
use libtypedtmpl::{
    collapse, Expander, HtmlCollapse, NoSources, Record, Registry, Scope, Templates, Value,
};

const SOURCE: &str = r#"# [Gallery] companion
- [Shot] inbox
  - title: Inbox
  - [-] src: /img/inbox.png
- [Shot] thread
  - title: A thread
  - [-] src: /img/thread.png

# [Template] Phone
- src: Link
- title: String
- [Content]
  ```html
  <figure><img src="{{ [-] this.src }}" alt="{{ [abc] this.title }}"></figure>
  ```

# [Section] screens
- [Content]
  ```html
  <section>{{ children }}</section>
  ```
- [List] gallery source={{ [1.] companion }} item_alias=shot
  - [Item]
    - [Phone] src={{ [-] shot.src }} title={{ [abc] shot.title }}
  - [Empty]
    - [Content]
      ```html
      <p>No screenshots.</p>
      ```
"#;

fn main() {
    let doc = parse_with(SOURCE, &ParseOptions::strict()).expect("the example parses");
    let nodes = &doc.documents[0].nodes;
    let mut scope = Scope::new();
    scope.bind(
        "companion",
        Value::Record(Record::from_object(&nodes[0]).expect("the gallery reads")),
    );
    let templates = Templates::from_stream(&doc).expect("the types read");
    let registry = Registry::new();
    let mut cx = Expander::new(&registry, &templates, &NoSources);
    let tree = cx
        .expand_children(&nodes[2], &scope)
        .expect("the section expands");

    for node in &tree {
        print!("{node}");
    }
    let mut html = String::new();
    collapse(&tree, &mut HtmlCollapse::new(), &mut html).expect("the tree collapses");
    println!("\n{html}");
}
