use std::marker::PhantomData;

use serde::de::DeserializeOwned;
use serde_json::{from_value, Value};

pub trait Jsoth {
    type Output;

    fn act(&self, value: &Value, output: &mut Vec<Self::Output>);
}

pub struct Identity;

impl Jsoth for Identity {
    type Output = Value;

    fn act(&self, value: &Value, output: &mut Vec<Self::Output>) {
        output.push(value.clone());
    }
}

pub struct Index<J, I> {
    pub index: I,
    pub inner: J,
}

impl<J, O, I> Jsoth for Index<J, I> 
where
    I: serde_json::value::Index,
    J: Jsoth<Output = O>,
{
    type Output = O;
    
    fn act(&self, value: &Value, output: &mut Vec<Self::Output>) {
        if let Some(inner) = value.get(&self.index) {
            self.inner.act(inner, output);
        }
    }
}

pub struct Pointer<'p, J> {
    pub pointer: &'p str,
    pub inner: J,
}

impl<'p, J, O> Jsoth for Pointer<'p, J>
where
    J: Jsoth<Output = O>,
{
    type Output = O;

    fn act(&self, value: &Value, output: &mut Vec<Self::Output>) {
        if let Some(inner) = value.pointer(&self.pointer) {
            self.inner.act(inner, output);
        }
    }
}

pub struct Deserialize<T>(PhantomData<T>);

impl<T> Deserialize<T> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<T> Jsoth for Deserialize<T>
where
    T: DeserializeOwned,
{
    type Output = T;

    fn act(&self, value: &Value, output: &mut Vec<Self::Output>) {
        if let Ok(deserialized) = from_value(value.clone()) {
            output.push(deserialized);
        }
    }
}

pub struct ForEach<J>(pub J);

impl<J, O> Jsoth for ForEach<J>
where
    J: Jsoth<Output = O>,
{
    type Output = O;

    fn act(&self, value: &Value, mut output: &mut Vec<Self::Output>) {
        if let Some(values) = value.as_array() {
            for value in values {
                self.0.act(value, &mut output);
            }
        }
    }
}

pub fn parse<O>(jsoth: impl Jsoth<Output = O>, value: &Value) -> Vec<O> {
    let mut output = Vec::new();

    jsoth.act(value, &mut output);

    output
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::{parse, Deserialize, ForEach, Identity, Index, Pointer};

    #[test]
    fn test_index() {
        let jsoth = Index { index: "a", inner: Identity };

        let value = json!({
            "a": 1,
        });

        assert_eq!(vec![json!(1)], parse(jsoth, &value));
    }

    #[test]
    fn test_identity() {
        let value = json!({
            "a": 1,
            "b": [2, 3],
            "c": {
                "x": 0,
                "y": 1,
            },
        });

        let jsoth = Identity;

        assert_eq!(vec![value.clone()], parse(jsoth, &value));
    }

    #[test]
    fn test_pointer() {
        let value = json!({
            "a": 1,
            "b": [2, 3],
            "c": {
                "x": 0,
                "y": 1,
            },
        });

        let jsoth = Pointer { pointer: "/c/x", inner: Identity };

        assert_eq!(vec![json!(0)], parse(jsoth, &value));
    }

    #[test]
    fn test_deserialize() {
        let value = json!(1);

        let jsoth = Deserialize::<u8>::new();

        assert_eq!(vec![1], parse(jsoth, &value));
    }

    #[test]
    fn test_for_each() {
        let value = json!([1, 2, 3]);

        let jsoth = ForEach(Identity);

        assert_eq!(vec![json!(1), json!(2), json!(3)], parse(jsoth, &value));
    }

    #[test]
    fn test_complex() {
        let value = json!({
            "a": 1,
            "b": [2, 3],
            "c": {
                "x": 0,
                "y": 1,
                "z": [{"A": 1}, {"A": 2}]
            },
        });

        let jsoth = Pointer {
            pointer: "/c/z",
            inner: ForEach(Index { index: "A", inner: Deserialize::<u8>::new() }),
        };

        assert_eq!(vec![1, 2], parse(jsoth, &value));
    }
}
