//! Streamed objects as the native loaders leave them after `LoadBinary`.
//!
//! Fields are kept in native read order. A field's value is the value the
//! native loader stores (after any version conversion it performs); when the
//! loader converts the word it read, the file word is kept in `read`.

/// Link IDs are raw stream indices; 0xFFFFFFFF is NULL (`NiStream`
/// GetObjectFromLinkID 8803A0).
pub const NULL_LINK: u32 = u32::MAX;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    U8(u8),
    U16(u16),
    U32(u32),
    /// A binary32 word, never converted.
    F32(u32),
    Bytes(Vec<u8>),
    U16s(Vec<u16>),
    U32s(Vec<u32>),
    F32s(Vec<u32>),
    /// `NiStream::LoadCString` 880EE0: `None` for a length <= 0.
    Str(Option<Vec<u8>>),
    /// One link ID (pushed to the stream link-ID list).
    Link(u32),
    /// The IDs of one `ReadMultipleLinkIDs` (count pushed, then the IDs).
    Links(Vec<u32>),
    Record(Vec<Field>),
    List(Vec<Value>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field {
    pub name: &'static str,
    pub value: Value,
    /// The file word, when the native loader stored a converted value.
    pub read: Option<Value>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Fields(pub Vec<Field>);

impl Fields {
    pub fn put(&mut self, name: &'static str, value: Value) {
        self.0.push(Field { name, value, read: None });
    }

    pub fn put_converted(&mut self, name: &'static str, value: Value, read: Value) {
        self.0.push(Field { name, value, read: Some(read) });
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        self.0.iter().find(|f| f.name == name).map(|f| &f.value)
    }

    pub fn field(&self, name: &str) -> Option<&Field> {
        self.0.iter().find(|f| f.name == name)
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut Field> {
        self.0.iter_mut().find(|f| f.name == name)
    }

    pub fn u8(&self, name: &str) -> Option<u8> {
        match self.get(name)? {
            Value::U8(v) => Some(*v),
            _ => None,
        }
    }

    pub fn u16(&self, name: &str) -> Option<u16> {
        match self.get(name)? {
            Value::U16(v) => Some(*v),
            _ => None,
        }
    }

    pub fn u32(&self, name: &str) -> Option<u32> {
        match self.get(name)? {
            Value::U32(v) => Some(*v),
            _ => None,
        }
    }

    pub fn f32(&self, name: &str) -> Option<u32> {
        match self.get(name)? {
            Value::F32(v) => Some(*v),
            _ => None,
        }
    }

    pub fn f32s(&self, name: &str) -> Option<&[u32]> {
        match self.get(name)? {
            Value::F32s(v) => Some(v),
            _ => None,
        }
    }

    pub fn u16s(&self, name: &str) -> Option<&[u16]> {
        match self.get(name)? {
            Value::U16s(v) => Some(v),
            _ => None,
        }
    }

    pub fn link(&self, name: &str) -> Option<u32> {
        match self.get(name)? {
            Value::Link(v) => Some(*v),
            _ => None,
        }
    }

    pub fn links(&self, name: &str) -> Option<&[u32]> {
        match self.get(name)? {
            Value::Links(v) => Some(v),
            _ => None,
        }
    }

    pub fn string(&self, name: &str) -> Option<Option<&[u8]>> {
        match self.get(name)? {
            Value::Str(v) => Some(v.as_deref()),
            _ => None,
        }
    }

    pub fn record(&self, name: &str) -> Option<&[Field]> {
        match self.get(name)? {
            Value::Record(v) => Some(v),
            _ => None,
        }
    }

    pub fn list(&self, name: &str) -> Option<&[Value]> {
        match self.get(name)? {
            Value::List(v) => Some(v),
            _ => None,
        }
    }
}

/// One streamed object.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NifObject {
    /// The native class actually created (NiRTTI name of the created object,
    /// e.g. a stream type `NiCollisionSwitch` creates an `NiNode`).
    pub class: &'static str,
    /// The class name the stream named it by.
    pub stream_name: Vec<u8>,
    pub fields: Fields,
}

impl NifObject {
    pub fn is(&self, class: &str) -> bool {
        self.class == class
    }
}

pub fn f32_value(word: u32) -> f32 {
    f32::from_bits(word)
}
