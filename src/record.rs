//! Records: typedmd objects read as named fields.

use libtypedmd::{LeafForm, Node, NodeKind, Sigil, Text};

use crate::error::{Error, ErrorKind};
use crate::path::{NodePath, Segment};
use crate::value::{Date, Value};

/// A record: an optional name (an object's head) and fields in source
/// order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Record {
    name: Option<String>,
    /// The name as a value, so `get("name")` can answer it.
    name_value: Option<Box<Value>>,
    fields: Vec<(String, Value)>,
}

impl Record {
    pub fn new(name: Option<String>) -> Self {
        Self {
            name_value: name.clone().map(|n| Box::new(Value::Text(n))),
            name,
            fields: Vec::new(),
        }
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Append a field. A later field of the same name wins in [`get`](Self::get).
    pub fn push(&mut self, field: impl Into<String>, value: Value) {
        self.fields.push((field.into(), value));
    }

    /// The last field of that name; for `name` with no such field, the
    /// record's name as text.
    pub fn get(&self, field: &str) -> Option<&Value> {
        self.fields
            .iter()
            .rev()
            .find(|(name, _)| name == field)
            .map(|(_, value)| value)
            .or_else(|| {
                (field == "name")
                    .then_some(self.name_value.as_deref())
                    .flatten()
            })
    }

    pub fn fields(&self) -> &[(String, Value)] {
        &self.fields
    }

    /// Read a typedmd Object as a record:
    ///
    /// - its plain head is the name (empty is none; slots are an error);
    /// - each member is a field, its value typed by its sigil; a member with
    ///   child rows is a list of their records (child objects) or a record of
    ///   them (child members); a member whose value spans lines (a `- name: |`
    ///   block) and has no sigil is Markdown;
    /// - its child objects are the field `items`, always present;
    /// - prose, payloads and sinks are not read; a plain list row is an error.
    pub fn from_object(node: &Node) -> Result<Record, Error> {
        from_object_at(node, &NodePath::new())
    }
}

fn from_object_at(node: &Node, parent: &NodePath) -> Result<Record, Error> {
    let path = parent.child(Segment::of(node));
    let shape = |message: String| Error::new(ErrorKind::RecordShape, message).at(path.clone());
    let NodeKind::Object(object) = &node.kind else {
        return Err(shape("a record is read from an object".into()));
    };
    let name = plain(&object.head)
        .ok_or_else(|| {
            Error::new(ErrorKind::SlotInData, "a record's name may not hold slots").at(path.clone())
        })?
        .trim()
        .to_string();
    let mut record = Record::new(Some(name).filter(|n| !n.is_empty()));
    let mut items = Vec::new();
    for child in &node.nodes {
        match &child.kind {
            NodeKind::Member(member) => {
                if member.name == "items" {
                    return Err(shape(
                        "a member may not be named `items`; the object's child objects are its items"
                            .into(),
                    ));
                }
                let value = member_value(child, &path)?;
                record.push(member.name.clone(), value);
            }
            NodeKind::Object(_) => items.push(from_object_at(child, &path)?),
            NodeKind::Leaf(leaf) if leaf.form == LeafForm::Row => {
                return Err(shape(format!(
                    "the row `{}` is neither a member nor an object",
                    leaf.text
                )))
            }
            NodeKind::Leaf(_) => {} // prose: a note
            NodeKind::Fallback(f) => return Err(shape(format!("could not read `{}`", f.text))),
        }
    }
    record.push("items", Value::List(items));
    Ok(record)
}

/// A text with no slots.
fn plain(text: &Text) -> Option<&str> {
    text.as_literal()
}

fn member_value(node: &Node, parent: &NodePath) -> Result<Value, Error> {
    let NodeKind::Member(member) = &node.kind else {
        unreachable!("called on members")
    };
    let path = parent.child(Segment::of(node));
    let err = |kind, message: String| Error::new(kind, message).at(path.clone());
    let name = &member.name;
    match (&member.rhs, node.nodes.is_empty()) {
        (Some(rhs), true) => {
            let text = plain(rhs).ok_or_else(|| {
                err(
                    ErrorKind::SlotInData,
                    format!("`{name}` holds a slot; data is plain"),
                )
            })?;
            read_value(member.sigil, text)
                .map_err(|message| err(ErrorKind::BadValue, format!("`{name}`: {message}")))
        }
        (None, false) => {
            let objects = node.nodes.iter().all(|n| n.as_object().is_some());
            let members = node.nodes.iter().all(|n| n.as_member().is_some());
            match (member.sigil, objects, members) {
                (None | Some(Sigil::List), true, _) => Ok(Value::List(
                    node.nodes
                        .iter()
                        .map(|n| from_object_at(n, &path))
                        .collect::<Result<_, _>>()?,
                )),
                (None, _, true) => {
                    let mut record = Record::new(None);
                    for n in &node.nodes {
                        let m = n.as_member().expect("all members");
                        record.push(m.name.clone(), member_value(n, &path)?);
                    }
                    Ok(Value::Record(record))
                }
                (Some(sigil), _, true) => Err(err(
                    ErrorKind::RecordShape,
                    format!("`{name}` holds members, which take no sigil, not `[{}]`", sigil.token()),
                )),
                _ => Err(err(
                    ErrorKind::RecordShape,
                    format!("`{name}` holds a mix of rows; its children must all be objects or all members"),
                )),
            }
        }
        (None, true) => Err(err(ErrorKind::NoValue, format!("`{name}` has no value"))),
        (Some(_), false) => Err(err(
            ErrorKind::RecordShape,
            format!("`{name}` has both a value and child rows"),
        )),
    }
}

/// A member's text read as its sigil's type.
fn read_value(sigil: Option<Sigil>, text: &str) -> Result<Value, String> {
    let t = text.trim();
    match sigil {
        None if text.contains('\n') => Ok(Value::Markdown(text.to_string())),
        None | Some(Sigil::String) => Ok(Value::Text(text.to_string())),
        Some(Sigil::Number) if is_number(t) => Ok(Value::Number(t.to_string())),
        Some(Sigil::Number) => Err(format!("`{t}` is not a number")),
        Some(Sigil::Date) => Date::parse(t)
            .map(Value::Date)
            .ok_or_else(|| format!("`{t}` is not a date (YYYY-MM-DD)")),
        Some(Sigil::Link) if !t.is_empty() => Ok(Value::Link(t.to_string())),
        Some(Sigil::Link) => Err("an empty link".into()),
        Some(Sigil::Boolean) => match t {
            "true" => Ok(Value::Bool(true)),
            "false" => Ok(Value::Bool(false)),
            _ => Err(format!("`{t}` is not `true` or `false`")),
        },
        Some(Sigil::Currency) if !t.is_empty() => Ok(Value::Currency(t.to_string())),
        Some(Sigil::Currency) => Err("an empty amount".into()),
        Some(Sigil::List) => Err("`[1.]` takes child rows, not a value".into()),
    }
}

/// `-?[0-9]+(\.[0-9]+)?`.
fn is_number(s: &str) -> bool {
    let s = s.strip_prefix('-').unwrap_or(s);
    let (whole, frac) = match s.split_once('.') {
        Some((w, f)) => (w, Some(f)),
        None => (s, None),
    };
    let digits = |p: &str| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit());
    digits(whole) && frac.is_none_or(digits)
}
