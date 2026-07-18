use super::*;
use crate::test_support::example;

const SOURCE: &str = r#"app Demo

extern crate::backend
  Item(id:i64, name:str)
  load() -> [Item] ! Item

theme
  background #000000

qr docs "https://example.com/ice docs" correction=high version=normal(4)

state
  items:[Item] = []
  query = ""

on mount
  run load() -> loaded _ | failed _

on loaded(next)
  items = next

on failed(error)
  query = error.name

view
  input "Query" #query <-> query @w-full
"#;

#[path = "tests/application.rs"]
mod application;
#[path = "tests/basics.rs"]
mod basics;
#[path = "tests/flows.rs"]
mod flows;
#[path = "tests/operations.rs"]
mod operations;
#[path = "tests/values.rs"]
mod values;
