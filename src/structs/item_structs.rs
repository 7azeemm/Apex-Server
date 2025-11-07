use crate::extensions::fastnbt_ext::ValueExt;
use crate::utils::format_number;
use derive_new::new;
use fastnbt::Value;
use getset::Getters;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Default, Debug, Deserialize, Clone)]
pub struct ItemNbt {
    #[serde(rename = "Count")]
    count: u8,
    tag: Option<ItemTag>,
}

impl ItemNbt {
    pub fn count(&self) -> u64 { self.count as u64 }
    pub fn get_extra_map(&self) -> Option<&HashMap<String, Value>> {
        self.tag.as_ref()?.extra_attributes.as_ref()?.as_compound()
    }
    pub fn get_display_map(&self) -> Option<&HashMap<String, Value>> {
        self.tag.as_ref()?.display.as_ref()?.as_compound()
    }
}

#[derive(Default, Debug, Deserialize, Clone)]
pub struct ItemTag {
    display: Option<Value>,
    #[serde(rename = "ExtraAttributes")]
    extra_attributes: Option<Value>,
}

#[derive(Default, Debug, Clone)]
pub struct ItemValue {
    info: Vec<String>,
    modifiers_value: u64,
    base_value: u64,
    value: u64,
    include_prices: bool,
    include_cosmetics: bool
}

impl ItemValue {
    pub fn new(include_prices: bool, include_cosmetics: bool) -> Self {
        Self { info: Vec::default(), modifiers_value: 0, base_value: 0, value: 0, include_prices, include_cosmetics }
    }

    pub fn add(&mut self, line: &str, price: Option<u64>, count: u64) {
        let value = price.unwrap_or(0) * count;
        self.modifiers_value += value;
        self.value = self.modifiers_value + self.base_value;
        self.info.push(match self.include_prices && price.is_some() {
            true => format!("{line} ({} coins)", format_number(value)),
            false => line.to_owned()
        });
    }

    pub fn add_cosmetic(&mut self, line: &str, price: Option<u64>) {
        match self.include_cosmetics {
            true => self.add(line, price, 1),
            false => {
                let value = price.unwrap_or(0);
                self.info.push(match self.include_prices && price.is_some() {
                    true => format!("{line} ({} coins) (Cosmetic, not included in the value estimation!)", format_number(value)),
                    false => line.to_owned()
                });
            }
        }
    }

    pub fn add_line(&mut self, line: &str) {
        self.info.push(line.to_owned());
    }

    pub fn add_value(&mut self, price: u64) {
        self.modifiers_value += price;
        self.value = self.modifiers_value + self.base_value;
    }

    pub fn modifiers_value(&self) -> u64 { self.modifiers_value }
    pub fn value(&self) -> u64 { self.value }
    pub fn info(&self) -> Vec<String> { self.info.clone() }

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

impl Modifier {
    pub fn new(name: &str, count: u64, price: u64) -> Self {
        Self { name: name.to_owned(), count, price }
    }

    pub fn count(&self) -> u64 { self.count }
    pub fn get_price(&self) -> u64 { &self.price * self.count }
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

#[derive(new, Getters)]
#[getset(get = "pub")]
pub struct ModifierContext<'a> {
    item_id: &'a str,
    item_nbt: &'a ItemNbt,
}