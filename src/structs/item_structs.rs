use crate::extensions::fastnbt_ext::ValueExt;
use fastnbt::Value;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Default, Debug, Deserialize, Clone)]
pub struct ItemNbt {
    #[serde(rename = "Count")]
    pub count: u8,
    pub tag: Option<ItemTag>,
}

impl ItemNbt {
    pub fn get_extra_map(&self) -> Option<&HashMap<String, Value>> {
        self.tag.as_ref()?.extra_attributes.as_ref()?.as_compound()
    }
}

#[derive(Default, Debug, Deserialize, Clone)]
pub struct ItemTag {
    pub display: Option<Value>,
    #[serde(rename = "ExtraAttributes")]
    pub extra_attributes: Option<Value>,
}

#[derive(Default, Debug, Clone)]
pub struct ItemValue {
    info: Vec<String>,
    modifiers_value: u64,
    base_value: u64,
    value: u64,
}

impl ItemValue {
    pub fn new() -> Self {
        Self { info: Vec::new(), modifiers_value: 0, base_value: 0, value: 0 }
    }

    pub fn add(&mut self, line: &str) {
        self.info.push(line.to_owned());
    }

    pub fn add_v(&mut self, line: &str, price: Option<u64>, count: u64) {
        self.info.push(line.to_owned());
        self.modifiers_value += price.unwrap_or(0) * count;
        self.value = self.modifiers_value + self.base_value;
    }

    pub fn add_value(&mut self, price: u64) {
        self.modifiers_value += price;
        self.value = self.modifiers_value + self.base_value;
    }

    pub fn modifiers_value(&self) -> u64 { self.modifiers_value }
    pub fn base_value(&self) -> u64 { self.base_value }
    pub fn value(&self) -> u64 { self.value }
    pub fn info(&self) -> Vec<String> { self.info.clone() }

    pub fn set_modifiers_value(&mut self, value: u64) {
        self.modifiers_value = value;
        self.value = self.modifiers_value + self.base_value;
    }
    pub fn set_base_value(&mut self, value: u64) {
        self.base_value = value;
        self.value = self.modifiers_value + self.base_value;
    }
}

#[derive(Debug, Clone)]
pub enum ModifierItem {
    Modifier(Modifier),
    Group(ModifierGroup),
}

#[derive(Debug, Clone)]
pub struct Modifier {
    name: String,
    count: u64,
    price: u64,
}

#[derive(Debug, Clone)]
pub struct ModifierGroup {
    name: String,
    modifiers: Vec<ModifierItem>,
}

impl ModifierGroup {
    pub fn new(name: &str) -> Self {
        Self { name: name.to_owned(), modifiers: Vec::new() }
    }
    pub fn add(&mut self, name: &str, count: u64, price: u64) {
        &self.modifiers.push(ModifierItem::Modifier(Modifier::new(name, count, price)));
    }
    pub fn add_group(&mut self, group: ModifierGroup) {
        &self.modifiers.push(ModifierItem::Group(group));
    }
}

impl Modifier {
    pub fn new(name: &str, count: u64, price: u64) -> Self {
        Self { name: name.to_owned(), count, price }
    }

    pub fn count(&self) -> u64 { self.count }
    pub fn get_price(&self) -> u64 { &self.price * self.count }
}

pub struct ModifierContext<'a> {
    item_id: &'a str,
    item_nbt: &'a ItemNbt,
}

impl<'a> ModifierContext<'a> {
    pub fn new(item_id: &'a str, item_nbt: &'a ItemNbt) -> ModifierContext<'a> {
        Self { item_id, item_nbt }
    }

    pub fn item_id(&self) -> &str { &self.item_id }
    pub fn item_nbt(&self) -> &ItemNbt { &self.item_nbt }
}