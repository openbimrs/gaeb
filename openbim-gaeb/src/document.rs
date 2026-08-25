use std::io::{Read, Write};

use crate::{parser, parser::QuantityEdit, Diagnostic, Error, Item, Metadata};

const UTF8_BOM: &[u8] = b"\xEF\xBB\xBF";

fn strip_utf8_bom(bytes: &[u8]) -> &[u8] {
    bytes.strip_prefix(UTF8_BOM).unwrap_or(bytes)
}

fn looks_like_xml(bytes: &[u8]) -> bool {
    strip_utf8_bom(bytes)
        .iter()
        .copied()
        .find(|byte| !byte.is_ascii_whitespace())
        == Some(b'<')
}

/// An owned GAEB document with lossless source bytes and extracted domain views.
///
/// Unknown elements, comments, prefixes, whitespace, and attribute order remain
/// in `bytes`. Reading and writing an unchanged document is therefore byte exact.
#[derive(Debug, Clone)]
pub struct Document {
    bytes: Vec<u8>,
    metadata: Metadata,
    diagnostics: Vec<Diagnostic>,
    items: Vec<Item>,
    quantity_edits: Vec<QuantityEdit>,
}

impl Document {
    /// Parse an owned, lossless GAEB view from XML bytes.
    pub fn parse(source: impl AsRef<[u8]>) -> Result<Self, Error> {
        let bytes = source.as_ref();
        if !looks_like_xml(bytes) {
            return Err(Error::NotXml);
        }
        let xml = strip_utf8_bom(bytes);
        let offset = bytes.len() - xml.len();
        let parsed = parser::parse(xml, offset)?;
        Ok(Self {
            bytes: bytes.to_vec(),
            metadata: parsed.metadata,
            diagnostics: parsed.diagnostics,
            items: parsed.items,
            quantity_edits: parsed.quantity_edits,
        })
    }

    /// Read a complete GAEB document from a stream.
    pub fn read_from(mut reader: impl Read) -> Result<Self, Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes)?;
        Self::parse(bytes)
    }

    /// Detection evidence and GAEB header metadata.
    #[must_use]
    pub const fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    /// Non-fatal evidence conflicts and unsupported declarations.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Common views of schema-positioned BoQ `Itemlist/Item` elements in document order.
    #[must_use]
    pub fn items(&self) -> &[Item] {
        &self.items
    }

    /// Find an item by exact GAEB `ID`.
    ///
    /// Returns the first match. Mutating methods reject duplicate IDs instead of
    /// choosing one silently.
    #[must_use]
    pub fn item(&self, id: &str) -> Option<&Item> {
        if id.is_empty() {
            return None;
        }
        self.items.iter().find(|item| item.id == id)
    }

    /// Current XML bytes, including the original BOM and all unknown content.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Consume the document and return its current XML bytes.
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    /// Write the current XML bytes without reformatting or tree regeneration.
    pub fn write_to(&self, mut writer: impl Write) -> Result<(), Error> {
        writer.write_all(&self.bytes)?;
        Ok(())
    }

    /// Replace an item's direct `<Qty>` value while preserving all other bytes.
    ///
    /// The edit is atomic: invalid decimals, missing/duplicate IDs, missing
    /// quantity fields, fragmented or mixed-content values, and reparsing failures
    /// leave this document unchanged.
    pub fn set_item_quantity(&mut self, item_id: &str, quantity: &str) -> Result<(), Error> {
        if item_id.is_empty() {
            return Err(Error::ItemNotFound(item_id.to_owned()));
        }
        if !is_xsd_decimal(quantity) {
            return Err(Error::InvalidDecimal(quantity.to_owned()));
        }
        let matches: Vec<usize> = self
            .items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| (item.id == item_id).then_some(index))
            .collect();
        let index = match matches.as_slice() {
            [] => return Err(Error::ItemNotFound(item_id.to_owned())),
            [index] => *index,
            _ => return Err(Error::AmbiguousItem(item_id.to_owned())),
        };
        let range = match &self.quantity_edits[index] {
            QuantityEdit::Missing => return Err(Error::QuantityMissing(item_id.to_owned())),
            QuantityEdit::NotEditable => {
                return Err(Error::QuantityNotEditable(item_id.to_owned()));
            }
            QuantityEdit::Editable(range) => range.clone(),
        };

        let mut edited = Vec::with_capacity(self.bytes.len() - range.len() + quantity.len());
        edited.extend_from_slice(&self.bytes[..range.start]);
        edited.extend_from_slice(quantity.as_bytes());
        edited.extend_from_slice(&self.bytes[range.end..]);
        let reparsed = Self::parse(edited)?;
        *self = reparsed;
        Ok(())
    }
}

fn is_xsd_decimal(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.is_empty() || value.trim() != value {
        return false;
    }
    let mut index = usize::from(matches!(bytes[0], b'+' | b'-'));
    if index == bytes.len() {
        return false;
    }
    let mut digits = 0;
    let mut dots = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'0'..=b'9' => digits += 1,
            b'.' if dots == 0 => dots += 1,
            _ => return false,
        }
        index += 1;
    }
    digits > 0
}

#[cfg(test)]
mod tests {
    use super::{is_xsd_decimal, looks_like_xml, strip_utf8_bom};

    #[test]
    fn local_xml_detection_handles_bom_and_whitespace() {
        let xml = b"\xEF\xBB\xBF \r\n<GAEB/>";
        assert!(looks_like_xml(xml));
        assert_eq!(strip_utf8_bom(xml), b" \r\n<GAEB/>");
        assert!(!looks_like_xml(b"PK\x03\x04"));
    }

    #[test]
    fn decimal_lexical_space_matches_xml_schema_shape() {
        for valid in ["0", "-1", "+1", "1.250", ".5", "5."] {
            assert!(is_xsd_decimal(valid), "{valid}");
        }
        for invalid in ["", "+", "-", " 1", "1 ", "1e3", "1,2", "."] {
            assert!(!is_xsd_decimal(invalid), "{invalid}");
        }
    }
}
