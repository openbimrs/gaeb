/// One bill-of-quantities category containing an item.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CategoryRef {
    pub id: Option<String>,
    pub outline_number: Option<String>,
    pub label: Option<String>,
}

/// A format-stable summary of a GAEB `<Item>`.
///
/// This intentionally exposes common fields rather than pretending every GAEB
/// phase has one giant identical item type. The original XML remains available
/// on [`crate::Document`] for unsupported fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub id: String,
    pub outline_number: Option<String>,
    pub quantity: Option<String>,
    pub unit: Option<String>,
    pub unit_price: Option<String>,
    pub total_price: Option<String>,
    pub description: Option<String>,
    pub category_path: Vec<CategoryRef>,
}
