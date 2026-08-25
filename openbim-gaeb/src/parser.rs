use std::{borrow::Cow, collections::HashSet, ops::Range, str};

use quick_xml::{
    escape::unescape,
    events::{BytesDecl, BytesStart, Event},
    name::ResolveResult,
    reader::NsReader,
};

use crate::{
    CategoryRef, Diagnostic, DiagnosticKind, Error, ExchangePhase, GaebVersion, Item, Metadata,
};

#[derive(Debug, Clone)]
pub(crate) enum QuantityEdit {
    Missing,
    Editable(Range<usize>),
    NotEditable,
}

pub(crate) struct Parsed {
    pub metadata: Metadata,
    pub diagnostics: Vec<Diagnostic>,
    pub items: Vec<Item>,
    pub quantity_edits: Vec<QuantityEdit>,
}

#[derive(Debug, Default, Clone, Copy)]
struct MetadataDeclarations {
    version: usize,
    phase: usize,
}

#[derive(Default)]
struct CategoryBuilder {
    id: Option<String>,
    outline_number: Option<String>,
    label: Option<String>,
}

impl CategoryBuilder {
    fn as_ref(&self) -> CategoryRef {
        CategoryRef {
            id: self.id.clone(),
            outline_number: self.outline_number.clone(),
            label: self.label.clone(),
        }
    }
}

struct ItemBuilder {
    item_depth: usize,
    id: Option<String>,
    outline_number: Option<String>,
    quantity_seen: bool,
    quantity_ambiguous: bool,
    quantity: Option<String>,
    quantity_fragments: Vec<Range<usize>>,
    quantity_has_non_value_xml: bool,
    unit: Option<String>,
    unit_price: Option<String>,
    total_price: Option<String>,
    description: String,
}

impl ItemBuilder {
    fn new(start: &BytesStart<'_>, item_depth: usize) -> Result<Self, Error> {
        Ok(Self {
            item_depth,
            id: attribute(start, b"ID")?,
            outline_number: attribute(start, b"RNoPart")?,
            quantity_seen: false,
            quantity_ambiguous: false,
            quantity: None,
            quantity_fragments: Vec::new(),
            quantity_has_non_value_xml: false,
            unit: None,
            unit_price: None,
            total_price: None,
            description: String::new(),
        })
    }

    fn start_quantity(&mut self) {
        if self.quantity_seen {
            self.quantity_ambiguous = true;
            self.quantity = None;
            self.quantity_fragments.clear();
            self.quantity_has_non_value_xml = true;
        }
        self.quantity_seen = true;
    }

    fn capture_quantity(&mut self, value: &str, range: Option<Range<usize>>) {
        if self.quantity_ambiguous {
            return;
        }
        append_optional_raw(&mut self.quantity, value);
        match range {
            Some(range) => self.quantity_fragments.push(range),
            None => self.quantity_has_non_value_xml = true,
        }
    }

    fn invalidate_quantity_value(&mut self) {
        self.quantity_seen = true;
        self.quantity_ambiguous = true;
        self.quantity = None;
        self.quantity_fragments.clear();
        self.quantity_has_non_value_xml = true;
    }

    fn block_quantity_edit(&mut self) {
        self.quantity_has_non_value_xml = true;
    }

    fn finish(self, categories: &[CategoryBuilder]) -> (Item, QuantityEdit) {
        let quantity = if self.quantity_ambiguous {
            None
        } else {
            normalize_optional(self.quantity)
        };
        let quantity_edit = if !self.quantity_seen {
            QuantityEdit::Missing
        } else if self.quantity_ambiguous {
            QuantityEdit::NotEditable
        } else if quantity.is_none() {
            QuantityEdit::Missing
        } else if !self.quantity_has_non_value_xml && self.quantity_fragments.len() == 1 {
            QuantityEdit::Editable(self.quantity_fragments[0].clone())
        } else {
            QuantityEdit::NotEditable
        };
        let description = (!self.description.is_empty()).then_some(self.description);
        (
            Item {
                id: self.id.unwrap_or_default(),
                outline_number: self.outline_number,
                quantity,
                unit: normalize_optional(self.unit),
                unit_price: normalize_optional(self.unit_price),
                total_price: normalize_optional(self.total_price),
                description,
                category_path: categories.iter().map(CategoryBuilder::as_ref).collect(),
            },
            quantity_edit,
        )
    }
}

#[derive(Debug)]
struct PathElement {
    local: String,
    gaeb: bool,
}

impl PathElement {
    fn is_gaeb(&self, local: &str) -> bool {
        self.gaeb && self.local == local
    }
}

pub(crate) fn parse(source: &[u8], source_offset: usize) -> Result<Parsed, Error> {
    let source_text = str::from_utf8(source)
        .map_err(|error| Error::Xml(format!("GAEB input is not UTF-8: {error}")))?;
    validate_xml_chars(source_text, "XML source")?;

    let mut reader = NsReader::from_reader(source);
    reader.config_mut().trim_text(false);

    let mut metadata = None;
    let mut root_namespace = None;
    let mut root_closed = false;
    let mut declaration_seen = false;
    let mut prolog_content_seen = false;
    let mut declarations = MetadataDeclarations::default();
    let mut diagnostics = Vec::new();
    let mut items = Vec::new();
    let mut quantity_edits = Vec::new();
    let mut path: Vec<PathElement> = Vec::new();
    let mut categories: Vec<CategoryBuilder> = Vec::new();
    let mut current_item: Option<ItemBuilder> = None;

    loop {
        match reader.read_resolved_event() {
            Ok((resolved, Event::Start(start))) => {
                prolog_content_seen = true;
                let namespace = resolved_namespace(resolved)?;
                validate_attributes(&reader, &start)?;
                let local = local_name(&start)?;
                if metadata.is_some() && path.is_empty() {
                    return Err(Error::Xml("multiple XML root elements".into()));
                }
                if metadata.is_none() {
                    let namespace = namespace.as_deref().ok_or(Error::NotGaeb)?;
                    let namespace = str::from_utf8(namespace)
                        .map_err(|error| Error::Xml(format!("non-UTF-8 namespace: {error}")))?;
                    if local != "GAEB" || !is_supported_gaeb_namespace(namespace) {
                        return Err(Error::NotGaeb);
                    }
                    root_namespace = Some(namespace.as_bytes().to_vec());
                    metadata = Some(Metadata::new(namespace.to_owned()));
                }
                if current_item
                    .as_ref()
                    .is_some_and(|item| direct_quantity(&path, item.item_depth))
                {
                    current_item
                        .as_mut()
                        .expect("quantity owner checked above")
                        .invalidate_quantity_value();
                }
                let gaeb = namespace.as_deref() == root_namespace.as_deref();
                path.push(PathElement {
                    local: local.clone(),
                    gaeb,
                });
                track_metadata_declaration(
                    &path,
                    metadata
                        .as_ref()
                        .expect("root established before metadata declaration")
                        .namespace
                        .as_str(),
                    &mut declarations,
                    &mut diagnostics,
                );
                if gaeb
                    && local == "BoQCtgy"
                    && direct_boqbody_category(
                        &path,
                        metadata
                            .as_ref()
                            .expect("root established before category")
                            .namespace
                            .as_str(),
                    )
                {
                    categories.push(CategoryBuilder {
                        id: attribute(&start, b"ID")?,
                        outline_number: attribute(&start, b"RNoPart")?,
                        label: None,
                    });
                } else if gaeb
                    && local == "Item"
                    && direct_itemlist_item(
                        &path,
                        metadata
                            .as_ref()
                            .expect("root established before item")
                            .namespace
                            .as_str(),
                    )
                {
                    if current_item.is_some() {
                        return Err(Error::Xml(
                            "nested GAEB Item elements are unsupported".into(),
                        ));
                    }
                    current_item = Some(ItemBuilder::new(&start, path.len())?);
                } else if gaeb && local == "Qty" {
                    if let Some(item) = current_item.as_mut() {
                        if direct_item_child(&path, item.item_depth) {
                            item.start_quantity();
                        }
                    }
                }
            }
            Ok((resolved, Event::Empty(start))) => {
                prolog_content_seen = true;
                let namespace = resolved_namespace(resolved)?;
                validate_attributes(&reader, &start)?;
                let local = local_name(&start)?;
                if metadata.is_some() && path.is_empty() {
                    return Err(Error::Xml("multiple XML root elements".into()));
                }
                if metadata.is_none() {
                    let namespace = namespace.as_deref().ok_or(Error::NotGaeb)?;
                    let namespace = str::from_utf8(namespace)
                        .map_err(|error| Error::Xml(format!("non-UTF-8 namespace: {error}")))?;
                    if local != "GAEB" || !is_supported_gaeb_namespace(namespace) {
                        return Err(Error::NotGaeb);
                    }
                    root_namespace = Some(namespace.as_bytes().to_vec());
                    metadata = Some(Metadata::new(namespace.to_owned()));
                    root_closed = true;
                }
                if current_item
                    .as_ref()
                    .is_some_and(|item| direct_quantity(&path, item.item_depth))
                {
                    current_item
                        .as_mut()
                        .expect("quantity owner checked above")
                        .invalidate_quantity_value();
                }
                let gaeb = namespace.as_deref() == root_namespace.as_deref();
                path.push(PathElement {
                    local: local.clone(),
                    gaeb,
                });
                track_metadata_declaration(
                    &path,
                    metadata
                        .as_ref()
                        .expect("root established before metadata declaration")
                        .namespace
                        .as_str(),
                    &mut declarations,
                    &mut diagnostics,
                );
                if gaeb
                    && local == "Item"
                    && direct_itemlist_item(
                        &path,
                        metadata
                            .as_ref()
                            .expect("root established before empty item")
                            .namespace
                            .as_str(),
                    )
                {
                    if current_item.is_some() {
                        return Err(Error::Xml(
                            "nested GAEB Item elements are unsupported".into(),
                        ));
                    }
                    let builder = ItemBuilder::new(&start, path.len())?;
                    let (item, edit) = builder.finish(&categories);
                    items.push(item);
                    quantity_edits.push(edit);
                } else if gaeb && local == "Qty" {
                    if let Some(item) = current_item.as_mut() {
                        if direct_item_child(&path, item.item_depth) {
                            item.invalidate_quantity_value();
                        }
                    }
                }
                path.pop();
            }
            Ok((_, Event::Text(text))) => {
                prolog_content_seen = true;
                let raw = str::from_utf8(text.as_ref())
                    .map_err(|error| Error::Xml(format!("non-UTF-8 XML text: {error}")))?;
                if path.is_empty() {
                    if raw.trim().is_empty() {
                        continue;
                    }
                    return Err(if metadata.is_none() {
                        Error::NotGaeb
                    } else {
                        Error::Xml("non-whitespace content outside the XML root".into())
                    });
                }
                let normalized = normalize_xml_line_endings(raw);
                let decoded = unescape(normalized.as_ref())
                    .map_err(|error| Error::Xml(format!("invalid XML entity: {error}")))?;
                validate_xml_chars(decoded.as_ref(), "decoded XML text")?;
                let end = reader.buffer_position() as usize + source_offset;
                let range = end.checked_sub(text.as_ref().len()).map(|start| start..end);
                capture_text(
                    &path,
                    decoded.as_ref(),
                    range,
                    metadata.as_mut().expect("root established before text"),
                    &mut categories,
                    declarations,
                    current_item.as_mut(),
                );
            }
            Ok((_, Event::CData(text))) => {
                prolog_content_seen = true;
                if path.is_empty() {
                    return Err(if metadata.is_none() {
                        Error::NotGaeb
                    } else {
                        Error::Xml("CDATA outside the XML root".into())
                    });
                }
                let value = str::from_utf8(text.as_ref())
                    .map_err(|error| Error::Xml(format!("non-UTF-8 CDATA: {error}")))?;
                let normalized = normalize_xml_line_endings(value);
                validate_xml_chars(normalized.as_ref(), "CDATA")?;
                let event_end = reader.buffer_position() as usize + source_offset;
                let range = event_end.checked_sub(3).and_then(|content_end| {
                    content_end
                        .checked_sub(text.as_ref().len())
                        .map(|start| start..content_end)
                });
                capture_text(
                    &path,
                    normalized.as_ref(),
                    range,
                    metadata.as_mut().expect("root established before CDATA"),
                    &mut categories,
                    declarations,
                    current_item.as_mut(),
                );
            }
            Ok((resolved, Event::End(end))) => {
                prolog_content_seen = true;
                resolved_namespace(resolved)?;
                validate_qname(end.name().as_ref(), "element")?;
                let local = str::from_utf8(end.local_name().as_ref())
                    .map_err(|error| Error::Xml(format!("non-UTF-8 element name: {error}")))?
                    .to_owned();
                let gaeb = path.last().is_some_and(|element| element.gaeb);
                if gaeb
                    && local == "Item"
                    && direct_itemlist_item(
                        &path,
                        metadata
                            .as_ref()
                            .expect("root established before item end")
                            .namespace
                            .as_str(),
                    )
                {
                    if let Some(builder) = current_item.take() {
                        let (item, edit) = builder.finish(&categories);
                        items.push(item);
                        quantity_edits.push(edit);
                    }
                } else if gaeb
                    && local == "BoQCtgy"
                    && direct_boqbody_category(
                        &path,
                        metadata
                            .as_ref()
                            .expect("root established before category end")
                            .namespace
                            .as_str(),
                    )
                {
                    categories.pop();
                }
                path.pop();
                if path.is_empty() {
                    root_closed = true;
                }
            }
            Ok((_, Event::Decl(declaration))) => {
                if declaration_seen || prolog_content_seen || metadata.is_some() {
                    return Err(Error::Xml(
                        "XML declaration must appear at most once at the start of the document"
                            .into(),
                    ));
                }
                validate_declaration(&declaration)?;
                declaration_seen = true;
            }
            Ok((_, Event::DocType(_))) => {
                return Err(Error::Xml(
                    "DOCTYPE declarations are not supported in GAEB documents".into(),
                ));
            }
            Ok((_, Event::Comment(comment))) => {
                prolog_content_seen = true;
                validate_comment(comment.as_ref())?;
                if current_item
                    .as_ref()
                    .is_some_and(|item| direct_quantity(&path, item.item_depth))
                {
                    current_item
                        .as_mut()
                        .expect("quantity owner checked above")
                        .block_quantity_edit();
                }
            }
            Ok((_, Event::PI(instruction))) => {
                prolog_content_seen = true;
                validate_processing_instruction(instruction.as_ref())?;
                if current_item
                    .as_ref()
                    .is_some_and(|item| direct_quantity(&path, item.item_depth))
                {
                    current_item
                        .as_mut()
                        .expect("quantity owner checked above")
                        .block_quantity_edit();
                }
            }
            Ok((_, Event::Eof)) => {
                if !path.is_empty() {
                    return Err(Error::Xml(format!(
                        "unexpected end of file with unclosed <{}>",
                        path.last()
                            .map(|element| element.local.as_str())
                            .unwrap_or("unknown")
                    )));
                }
                break;
            }
            Err(error) => return Err(Error::Xml(error.to_string())),
        }
    }

    let mut metadata = metadata.ok_or(Error::NotGaeb)?;
    if !root_closed {
        return Err(Error::Xml("XML root was not closed".into()));
    }
    normalize_metadata(&mut metadata);
    finalize_detection(&mut metadata, &mut diagnostics);
    for item in &items {
        if item.id.is_empty() {
            diagnostics.push(Diagnostic::new(
                DiagnosticKind::MissingItemId,
                "GAEB Item has no ID attribute",
            ));
        }
    }

    Ok(Parsed {
        metadata,
        diagnostics,
        items,
        quantity_edits,
    })
}

fn is_xml_name_start(character: char, allow_colon: bool) -> bool {
    matches!(character, 'A'..='Z' | '_' | 'a'..='z')
        || (allow_colon && character == ':')
        || matches!(
            character as u32,
            0xC0..=0xD6
                | 0xD8..=0xF6
                | 0xF8..=0x2FF
                | 0x370..=0x37D
                | 0x37F..=0x1FFF
                | 0x200C..=0x200D
                | 0x2070..=0x218F
                | 0x2C00..=0x2FEF
                | 0x3001..=0xD7FF
                | 0xF900..=0xFDCF
                | 0xFDF0..=0xFFFD
                | 0x10000..=0xEFFFF
        )
}

fn is_xml_name_char(character: char, allow_colon: bool) -> bool {
    is_xml_name_start(character, allow_colon)
        || matches!(character, '-' | '.' | '0'..='9' | '\u{B7}')
        || matches!(character as u32, 0x300..=0x36F | 0x203F..=0x2040)
}

fn validate_xml_name(name: &str, allow_colon: bool, context: &str) -> Result<(), Error> {
    let mut characters = name.chars();
    if !characters
        .next()
        .is_some_and(|character| is_xml_name_start(character, allow_colon))
        || !characters.all(|character| is_xml_name_char(character, allow_colon))
    {
        return Err(Error::Xml(format!("invalid XML {context} name `{name}`")));
    }
    Ok(())
}

fn validate_qname(name: &[u8], context: &str) -> Result<(), Error> {
    let name = str::from_utf8(name)
        .map_err(|error| Error::Xml(format!("non-UTF-8 XML {context} name: {error}")))?;
    let mut parts = name.split(':');
    let first = parts.next().expect("split always yields one part");
    let second = parts.next();
    if parts.next().is_some() {
        return Err(Error::Xml(format!("invalid XML {context} QName `{name}`")));
    }
    validate_xml_name(first, false, context)?;
    if let Some(local) = second {
        validate_xml_name(local, false, context)?;
    }
    Ok(())
}

fn validate_comment(comment: &[u8]) -> Result<(), Error> {
    let comment = str::from_utf8(comment)
        .map_err(|error| Error::Xml(format!("non-UTF-8 XML comment: {error}")))?;
    if comment.contains("--") || comment.ends_with('-') {
        return Err(Error::Xml(
            "XML comments cannot contain `--` or end with `-`".into(),
        ));
    }
    Ok(())
}

fn validate_processing_instruction(instruction: &[u8]) -> Result<(), Error> {
    let instruction = str::from_utf8(instruction)
        .map_err(|error| Error::Xml(format!("non-UTF-8 processing instruction: {error}")))?;
    let target_end = instruction
        .find(|character: char| character.is_ascii_whitespace())
        .unwrap_or(instruction.len());
    let target = &instruction[..target_end];
    if target.is_empty() {
        return Err(Error::Xml(
            "processing instruction target is missing".into(),
        ));
    }
    validate_xml_name(target, false, "processing instruction target")?;
    if target.eq_ignore_ascii_case("xml") {
        return Err(Error::Xml(
            "the processing instruction target `xml` is reserved".into(),
        ));
    }
    Ok(())
}

fn validate_xml_chars(value: &str, context: &str) -> Result<(), Error> {
    if let Some(character) = value.chars().find(|character| {
        !matches!(
            *character as u32,
            0x9 | 0xA | 0xD | 0x20..=0xD7FF | 0xE000..=0xFFFD | 0x10000..=0x10FFFF
        )
    }) {
        return Err(Error::Xml(format!(
            "{context} contains XML 1.0-forbidden character U+{:04X}",
            character as u32
        )));
    }
    Ok(())
}

fn normalize_xml_line_endings(value: &str) -> Cow<'_, str> {
    if !value.contains('\r') {
        return Cow::Borrowed(value);
    }
    let mut normalized = String::with_capacity(value.len());
    let mut characters = value.chars().peekable();
    while let Some(character) = characters.next() {
        if character == '\r' {
            if characters.peek() == Some(&'\n') {
                characters.next();
            }
            normalized.push('\n');
        } else {
            normalized.push(character);
        }
    }
    Cow::Owned(normalized)
}

fn normalize_xml_attribute_whitespace(value: &str) -> Cow<'_, str> {
    if !value
        .bytes()
        .any(|byte| matches!(byte, b'\t' | b'\n' | b'\r'))
    {
        return Cow::Borrowed(value);
    }
    let mut normalized = String::with_capacity(value.len());
    let mut characters = value.chars().peekable();
    while let Some(character) = characters.next() {
        match character {
            '\r' => {
                if characters.peek() == Some(&'\n') {
                    characters.next();
                }
                normalized.push(' ');
            }
            '\n' | '\t' => normalized.push(' '),
            _ => normalized.push(character),
        }
    }
    Cow::Owned(normalized)
}

const XML_NAMESPACE: &str = "http://www.w3.org/XML/1998/namespace";
const XMLNS_NAMESPACE: &str = "http://www.w3.org/2000/xmlns/";

fn validate_namespace_declaration(name: &[u8], value: &str) -> Result<(), Error> {
    let prefix = name.strip_prefix(b"xmlns:");
    if name == b"xmlns" {
        if matches!(value, XML_NAMESPACE | XMLNS_NAMESPACE) {
            return Err(Error::Xml(
                "the default namespace cannot use a reserved XML namespace name".into(),
            ));
        }
        return Ok(());
    }
    let Some(prefix) = prefix else {
        return Ok(());
    };
    if prefix == b"xmlns" {
        return Err(Error::Xml(
            "the reserved `xmlns` prefix cannot be declared".into(),
        ));
    }
    if value.is_empty() {
        return Err(Error::Xml(
            "a prefixed namespace declaration cannot have an empty value".into(),
        ));
    }
    if value == XMLNS_NAMESPACE {
        return Err(Error::Xml(
            "the XMLNS namespace name cannot be bound to a prefix".into(),
        ));
    }
    if prefix == b"xml" {
        if value != XML_NAMESPACE {
            return Err(Error::Xml(
                "the `xml` prefix must bind the XML namespace name".into(),
            ));
        }
    } else if value == XML_NAMESPACE {
        return Err(Error::Xml(
            "the XML namespace name can only be bound to the `xml` prefix".into(),
        ));
    }
    Ok(())
}

fn validate_attributes(reader: &NsReader<&[u8]>, start: &BytesStart<'_>) -> Result<(), Error> {
    validate_qname(start.name().as_ref(), "element")?;
    if start.name().as_ref().starts_with(b"xmlns:") {
        return Err(Error::Xml(
            "the reserved `xmlns` prefix cannot name an element".into(),
        ));
    }
    let mut expanded_names = HashSet::new();
    for result in start.attributes().with_checks(true) {
        let attribute =
            result.map_err(|error| Error::Xml(format!("invalid attribute: {error}")))?;
        validate_qname(attribute.key.as_ref(), "attribute")?;
        let name = attribute.key.as_ref();
        let is_namespace_declaration = name == b"xmlns" || name.starts_with(b"xmlns:");
        let namespace = if is_namespace_declaration {
            None
        } else {
            resolved_namespace(reader.resolve_attribute(attribute.key).0)?
        };
        if !is_namespace_declaration
            && !expanded_names.insert((namespace, attribute.key.local_name().as_ref().to_vec()))
        {
            return Err(Error::Xml("duplicate expanded attribute name".into()));
        }
        let raw = str::from_utf8(attribute.value.as_ref())
            .map_err(|error| Error::Xml(format!("non-UTF-8 attribute: {error}")))?;
        let normalized = normalize_xml_attribute_whitespace(raw);
        let decoded = unescape(normalized.as_ref())
            .map_err(|error| Error::Xml(format!("invalid attribute entity: {error}")))?;
        validate_xml_chars(decoded.as_ref(), "decoded XML attribute")?;
        if is_namespace_declaration {
            validate_namespace_declaration(name, decoded.as_ref())?;
        }
    }
    Ok(())
}

fn validate_declaration(declaration: &BytesDecl<'_>) -> Result<(), Error> {
    let content = str::from_utf8(declaration.as_ref())
        .map_err(|error| Error::Xml(format!("non-UTF-8 XML declaration: {error}")))?;
    let start = BytesStart::from_content(content, 3);
    let mut last_rank = None;
    let mut count = 0_usize;
    for result in start.attributes().with_checks(true) {
        let attribute =
            result.map_err(|error| Error::Xml(format!("invalid XML declaration: {error}")))?;
        let key = attribute.key.as_ref();
        let value = str::from_utf8(attribute.value.as_ref())
            .map_err(|error| Error::Xml(format!("non-UTF-8 XML declaration: {error}")))?;
        let rank = match key {
            b"version" if count == 0 => {
                if value != "1.0" {
                    return Err(Error::Xml(format!(
                        "unsupported XML declaration version `{value}`"
                    )));
                }
                0
            }
            b"encoding" if count > 0 => {
                if !value.eq_ignore_ascii_case("UTF-8") {
                    return Err(Error::Xml(format!(
                        "unsupported XML declaration encoding `{value}`; expected UTF-8"
                    )));
                }
                1
            }
            b"standalone" if count > 0 => {
                if !matches!(value, "yes" | "no") {
                    return Err(Error::Xml(format!(
                        "invalid XML standalone value `{value}`"
                    )));
                }
                2
            }
            _ => {
                let key = String::from_utf8_lossy(key);
                return Err(Error::Xml(format!(
                    "unexpected XML declaration attribute `{key}`"
                )));
            }
        };
        if last_rank.is_some_and(|previous| rank <= previous) {
            return Err(Error::Xml(
                "XML declaration attributes are duplicated or out of order".into(),
            ));
        }
        last_rank = Some(rank);
        count += 1;
    }
    if count == 0 {
        return Err(Error::Xml(
            "XML declaration is missing the required version attribute".into(),
        ));
    }
    Ok(())
}

fn resolved_namespace(resolved: ResolveResult<'_>) -> Result<Option<Vec<u8>>, Error> {
    match resolved {
        ResolveResult::Unbound => Ok(None),
        ResolveResult::Bound(namespace) => {
            let raw = str::from_utf8(namespace.into_inner())
                .map_err(|error| Error::Xml(format!("non-UTF-8 namespace name: {error}")))?;
            let normalized = normalize_xml_attribute_whitespace(raw);
            let decoded = unescape(normalized.as_ref())
                .map_err(|error| Error::Xml(format!("invalid namespace entity: {error}")))?;
            validate_xml_chars(decoded.as_ref(), "decoded XML namespace name")?;
            Ok(Some(decoded.as_bytes().to_vec()))
        }
        ResolveResult::Unknown(prefix) => Err(Error::Xml(format!(
            "undeclared XML namespace prefix {:?}",
            String::from_utf8_lossy(&prefix)
        ))),
    }
}

fn local_name(start: &BytesStart<'_>) -> Result<String, Error> {
    str::from_utf8(start.local_name().as_ref())
        .map(str::to_owned)
        .map_err(|error| Error::Xml(format!("non-UTF-8 element name: {error}")))
}

fn attribute(start: &BytesStart<'_>, key: &[u8]) -> Result<Option<String>, Error> {
    for result in start.attributes().with_checks(true) {
        let attr = result.map_err(|error| Error::Xml(format!("invalid attribute: {error}")))?;
        if attr.key.as_ref() == key {
            let raw = str::from_utf8(attr.value.as_ref())
                .map_err(|error| Error::Xml(format!("non-UTF-8 attribute: {error}")))?;
            let normalized = normalize_xml_attribute_whitespace(raw);
            return unescape(normalized.as_ref())
                .map(|value| Some(value.into_owned()))
                .map_err(|error| Error::Xml(format!("invalid attribute entity: {error}")));
        }
    }
    Ok(None)
}

fn direct_item_child(path: &[PathElement], item_depth: usize) -> bool {
    path.len() == item_depth + 1 && path[item_depth - 1].is_gaeb("Item")
}

fn valid_boq_descendant_path(path: &[PathElement], namespace: &str) -> bool {
    if path.len() < 4
        || !path[0].is_gaeb("GAEB")
        || !expected_phase_parent(namespace).is_some_and(|parent| path[1].is_gaeb(parent))
        || !path[2].is_gaeb("BoQ")
    {
        return false;
    }
    path[3..].iter().enumerate().all(|(index, element)| {
        if index % 2 == 0 {
            element.is_gaeb("BoQBody")
        } else {
            element.is_gaeb("BoQCtgy")
        }
    })
}

fn direct_itemlist_item(path: &[PathElement], namespace: &str) -> bool {
    path.len() >= 6
        && path[path.len() - 2].is_gaeb("Itemlist")
        && path[path.len() - 1].is_gaeb("Item")
        && valid_boq_descendant_path(&path[..path.len() - 2], namespace)
}

fn direct_boqbody_category(path: &[PathElement], namespace: &str) -> bool {
    path.last()
        .is_some_and(|element| element.is_gaeb("BoQCtgy"))
        && valid_boq_descendant_path(path, namespace)
}

fn direct_quantity(path: &[PathElement], item_depth: usize) -> bool {
    direct_item_child(path, item_depth) && path[item_depth].is_gaeb("Qty")
}

fn in_direct_item_description(path: &[PathElement], item_depth: usize) -> bool {
    path.len() > item_depth
        && path[item_depth - 1].is_gaeb("Item")
        && path[item_depth].is_gaeb("Description")
        && !path[item_depth + 1..]
            .iter()
            .any(|element| element.is_gaeb("Item") || element.is_gaeb("SubDescr"))
}

fn direct_header_child(path: &[PathElement]) -> bool {
    path.len() == 3 && path[0].is_gaeb("GAEB") && path[1].is_gaeb("GAEBInfo")
}

fn expected_phase_parent(namespace: &str) -> Option<&'static str> {
    if namespace == "http://www.gaeb.de/GAEB_DA_XML/200407" {
        return Some("Award");
    }
    if namespace == "http://www.gaeb.de/GAEB_DA_XML/200706" {
        return Some("Order");
    }
    let product = namespace
        .strip_prefix("http://www.gaeb.de/GAEB_DA_XML/DA")?
        .split('/')
        .next()?;
    Some(match product {
        "31" => "QtyDeterm",
        "50" | "51" => "ElementalCosting",
        "61" => "GAEBInfo",
        "84P" => "SC_Evaluation",
        "89" | "89B" => "Invoice",
        "93" | "94" | "96" | "97" | "98" | "99" => "Order",
        "52" | "80" | "81" | "82" | "83" | "83Z" | "84" | "84Z" | "85" | "86" | "86ZE" | "86ZR"
        | "87" => "Award",
        _ => return None,
    })
}

fn direct_phase_child(path: &[PathElement], namespace: &str) -> bool {
    path.len() == 3
        && path[0].is_gaeb("GAEB")
        && path[2].is_gaeb("DP")
        && expected_phase_parent(namespace).is_some_and(|parent| path[1].is_gaeb(parent))
}

fn track_metadata_declaration(
    path: &[PathElement],
    namespace: &str,
    declarations: &mut MetadataDeclarations,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(current) = path.last() else {
        return;
    };
    if current.is_gaeb("Version") && direct_header_child(path) {
        declarations.version += 1;
        if declarations.version > 1 {
            diagnostics.push(Diagnostic::new(
                DiagnosticKind::DuplicateVersionDeclaration,
                "multiple root GAEBInfo Version declarations are ambiguous",
            ));
        }
    } else if current.is_gaeb("DP") && direct_phase_child(path, namespace) {
        declarations.phase += 1;
        if declarations.phase > 1 {
            diagnostics.push(Diagnostic::new(
                DiagnosticKind::DuplicatePhaseDeclaration,
                "multiple top-level DP declarations are ambiguous",
            ));
        }
    }
}

fn direct_category_child(path: &[PathElement], namespace: &str) -> bool {
    path.len() >= 6
        && path[path.len() - 2].is_gaeb("BoQCtgy")
        && valid_boq_descendant_path(&path[..path.len() - 1], namespace)
}

fn capture_text(
    path: &[PathElement],
    raw_value: &str,
    range: Option<Range<usize>>,
    metadata: &mut Metadata,
    categories: &mut [CategoryBuilder],
    declarations: MetadataDeclarations,
    item: Option<&mut ItemBuilder>,
) {
    let Some(current) = path.last() else {
        return;
    };
    if !current.gaeb {
        return;
    }

    if let Some(item) = item {
        if direct_item_child(path, item.item_depth) {
            match current.local.as_str() {
                "Qty" => item.capture_quantity(raw_value, range),
                "QU" => append_optional_raw(&mut item.unit, raw_value),
                "UP" => append_optional_raw(&mut item.unit_price, raw_value),
                "IT" => append_optional_raw(&mut item.total_price, raw_value),
                _ => {}
            }
        }
        if in_direct_item_description(path, item.item_depth) {
            append_words(&mut item.description, raw_value);
        }
        return;
    }

    let value = raw_value.trim();
    if value.is_empty() {
        return;
    }
    if current.local == "LblTx" && direct_category_child(path, metadata.namespace.as_str()) {
        if let Some(category) = categories.last_mut() {
            append_optional_words(&mut category.label, value);
        }
    }

    let in_header = direct_header_child(path);
    match current.local.as_str() {
        "Version" if in_header && declarations.version == 1 => {
            append_optional_raw(&mut metadata.version_text, raw_value)
        }
        "VersDate" if in_header => append_optional_raw(&mut metadata.version_date, raw_value),
        "Date" if in_header => append_optional_raw(&mut metadata.date, raw_value),
        "Time" if in_header => append_optional_raw(&mut metadata.time, raw_value),
        "ProgSystem" if in_header => append_optional_raw(&mut metadata.program_system, raw_value),
        "ProgName" if in_header => append_optional_raw(&mut metadata.program_name, raw_value),
        "DP" if direct_phase_child(path, metadata.namespace.as_str())
            && declarations.phase == 1 =>
        {
            append_optional_raw(&mut metadata.phase_code, raw_value)
        }
        _ => {}
    }
}

fn append_optional_raw(target: &mut Option<String>, value: &str) {
    target.get_or_insert_with(String::new).push_str(value);
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let value = value.trim();
        (!value.is_empty()).then(|| value.to_owned())
    })
}

fn normalize_metadata(metadata: &mut Metadata) {
    metadata.version_text = normalize_optional(metadata.version_text.take());
    metadata.version_date = normalize_optional(metadata.version_date.take());
    metadata.date = normalize_optional(metadata.date.take());
    metadata.time = normalize_optional(metadata.time.take());
    metadata.program_system = normalize_optional(metadata.program_system.take());
    metadata.program_name = normalize_optional(metadata.program_name.take());
    metadata.phase_code = normalize_optional(metadata.phase_code.take());
}

fn append_words(target: &mut String, value: &str) {
    for word in value.split_whitespace() {
        if !target.is_empty() {
            target.push(' ');
        }
        target.push_str(word);
    }
}

fn append_optional_words(target: &mut Option<String>, value: &str) {
    let target = target.get_or_insert_with(String::new);
    append_words(target, value);
}

const PHASES_3_2: &[&str] = &[
    "31", "52", "80", "81", "82", "83", "83Z", "84", "84Z", "85", "86", "86ZE", "86ZR", "87", "89",
    "93", "94", "96", "97",
];
const PHASES_3_3_AND_3_4: &[&str] = &[
    "31", "50", "51", "52", "61", "80", "81", "82", "83", "83Z", "84", "84P", "84Z", "85", "86",
    "86ZE", "86ZR", "87", "89", "89B", "93", "94", "96", "97", "98", "99",
];

fn is_supported_gaeb_namespace(namespace: &str) -> bool {
    namespace_evidence(namespace).is_some()
}

fn finalize_detection(metadata: &mut Metadata, diagnostics: &mut Vec<Diagnostic>) {
    let (namespace_version, namespace_phase) = namespace_evidence(&metadata.namespace)
        .map_or((None, None), |(version, phase)| (Some(version), phase));
    let declared_version = metadata
        .version_text
        .as_deref()
        .and_then(GaebVersion::from_text);
    let declared_phase = metadata
        .phase_code
        .as_deref()
        .and_then(ExchangePhase::from_code);

    metadata.namespace_version = namespace_version;
    metadata.declared_version = declared_version;
    metadata.namespace_phase = namespace_phase;
    metadata.declared_phase = declared_phase;
    metadata.version = namespace_version.or(declared_version);
    metadata.phase = namespace_phase.or(declared_phase);

    if let (Some(namespace), Some(declared)) = (namespace_version, declared_version) {
        if namespace != declared {
            diagnostics.push(Diagnostic::new(
                DiagnosticKind::VersionMismatch,
                format!("namespace identifies GAEB {namespace}, but <Version> declares {declared}"),
            ));
        }
    }
    if metadata.version_text.is_some() && declared_version.is_none() {
        diagnostics.push(Diagnostic::new(
            DiagnosticKind::UnsupportedVersion,
            format!(
                "unrecognized GAEB version {:?}",
                metadata.version_text.as_deref().unwrap_or_default()
            ),
        ));
    }
    if let Some(declared) = declared_phase {
        if !namespace_allows_phase(&metadata.namespace, declared) {
            diagnostics.push(Diagnostic::new(
                DiagnosticKind::PhaseMismatch,
                format!(
                    "namespace {:?} does not permit phase {declared}",
                    metadata.namespace
                ),
            ));
        }
    }
    if metadata.phase_code.is_some() && declared_phase.is_none() {
        diagnostics.push(Diagnostic::new(
            DiagnosticKind::UnknownPhase,
            format!(
                "unrecognized GAEB exchange phase {:?}",
                metadata.phase_code.as_deref().unwrap_or_default()
            ),
        ));
    }
}

fn namespace_evidence(namespace: &str) -> Option<(GaebVersion, Option<ExchangePhase>)> {
    match namespace {
        "http://www.gaeb.de/GAEB_DA_XML/200407" | "http://www.gaeb.de/GAEB_DA_XML/200706" => {
            return Some((GaebVersion::V3_1, None))
        }
        _ => {}
    }
    let rest = namespace.strip_prefix("http://www.gaeb.de/GAEB_DA_XML/DA")?;
    let (phase, version) = rest.split_once('/')?;
    let version = match version {
        "3.2" if PHASES_3_2.contains(&phase) => GaebVersion::V3_2,
        "3.3" if PHASES_3_3_AND_3_4.contains(&phase) => GaebVersion::V3_3,
        "3.4" if PHASES_3_3_AND_3_4.contains(&phase) => GaebVersion::V3_4Beta,
        _ => return None,
    };
    let phase = if matches!(phase, "50" | "51" | "84") {
        None
    } else {
        ExchangePhase::from_code(phase)
    };
    Some((version, phase))
}

fn namespace_allows_phase(namespace: &str, declared: ExchangePhase) -> bool {
    match namespace {
        "http://www.gaeb.de/GAEB_DA_XML/200407" => matches!(
            declared,
            ExchangePhase::X81
                | ExchangePhase::X82
                | ExchangePhase::X83
                | ExchangePhase::X84
                | ExchangePhase::X85
                | ExchangePhase::X86
                | ExchangePhase::X87
                | ExchangePhase::X88
        ),
        "http://www.gaeb.de/GAEB_DA_XML/200706" => matches!(
            declared,
            ExchangePhase::X93 | ExchangePhase::X94 | ExchangePhase::X96 | ExchangePhase::X97
        ),
        _ => {
            let Some(rest) = namespace.strip_prefix("http://www.gaeb.de/GAEB_DA_XML/DA") else {
                return false;
            };
            let Some((namespace_phase, _)) = rest.split_once('/') else {
                return false;
            };
            match namespace_phase {
                "50" => matches!(declared, ExchangePhase::X50_1 | ExchangePhase::X50_2),
                "51" => matches!(declared, ExchangePhase::X51_1 | ExchangePhase::X51_2),
                "84" => matches!(declared, ExchangePhase::X84 | ExchangePhase::X84Z),
                phase => declared.as_code() == phase,
            }
        }
    }
}
