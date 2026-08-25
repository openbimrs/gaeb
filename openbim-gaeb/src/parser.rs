use std::{ops::Range, str};

use quick_xml::{
    escape::unescape,
    events::{BytesStart, Event},
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
    id: Option<String>,
    outline_number: Option<String>,
    quantity_seen: bool,
    quantity: Option<String>,
    quantity_fragments: Vec<Range<usize>>,
    quantity_has_non_value_xml: bool,
    unit: Option<String>,
    unit_price: Option<String>,
    total_price: Option<String>,
    description: String,
}

impl ItemBuilder {
    fn new(start: &BytesStart<'_>) -> Result<Self, Error> {
        Ok(Self {
            id: attribute(start, b"ID")?,
            outline_number: attribute(start, b"RNoPart")?,
            quantity_seen: false,
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
            self.quantity_has_non_value_xml = true;
        }
        self.quantity_seen = true;
    }

    fn capture_quantity(&mut self, value: &str, range: Option<Range<usize>>) {
        append_optional_raw(&mut self.quantity, value);
        match range {
            Some(range) => self.quantity_fragments.push(range),
            None => self.quantity_has_non_value_xml = true,
        }
    }

    fn block_quantity_edit(&mut self) {
        self.quantity_has_non_value_xml = true;
    }

    fn finish(self, categories: &[CategoryBuilder]) -> (Item, QuantityEdit) {
        let quantity = normalize_optional(self.quantity);
        let quantity_edit = if !self.quantity_seen || quantity.is_none() {
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
    let mut reader = NsReader::from_reader(source);
    reader.config_mut().trim_text(false);

    let mut metadata = None;
    let mut root_namespace = None;
    let mut root_closed = false;
    let mut diagnostics = Vec::new();
    let mut items = Vec::new();
    let mut quantity_edits = Vec::new();
    let mut path: Vec<PathElement> = Vec::new();
    let mut categories: Vec<CategoryBuilder> = Vec::new();
    let mut current_item: Option<ItemBuilder> = None;

    loop {
        match reader.read_resolved_event() {
            Ok((resolved, Event::Start(start))) => {
                let namespace = resolved_namespace(resolved)?;
                let local = local_name(&start)?;
                if metadata.is_some() && path.is_empty() {
                    return Err(Error::Xml("multiple XML root elements".into()));
                }
                if metadata.is_none() {
                    let namespace = namespace.ok_or(Error::NotGaeb)?;
                    let namespace = str::from_utf8(namespace)
                        .map_err(|error| Error::Xml(format!("non-UTF-8 namespace: {error}")))?;
                    if local != "GAEB" || !is_supported_gaeb_namespace(namespace) {
                        return Err(Error::NotGaeb);
                    }
                    root_namespace = Some(namespace.as_bytes().to_vec());
                    metadata = Some(Metadata::new(namespace.to_owned()));
                }
                if direct_quantity(&path) {
                    if let Some(item) = current_item.as_mut() {
                        item.block_quantity_edit();
                    }
                }
                let gaeb = namespace == root_namespace.as_deref();
                path.push(PathElement {
                    local: local.clone(),
                    gaeb,
                });
                if gaeb && local == "BoQCtgy" {
                    categories.push(CategoryBuilder {
                        id: attribute(&start, b"ID")?,
                        outline_number: attribute(&start, b"RNoPart")?,
                        label: None,
                    });
                } else if gaeb && local == "Item" {
                    if current_item.is_some() {
                        return Err(Error::Xml(
                            "nested GAEB Item elements are unsupported".into(),
                        ));
                    }
                    current_item = Some(ItemBuilder::new(&start)?);
                } else if gaeb && local == "Qty" && direct_item_child(&path) {
                    if let Some(item) = current_item.as_mut() {
                        item.start_quantity();
                    }
                }
            }
            Ok((resolved, Event::Empty(start))) => {
                let namespace = resolved_namespace(resolved)?;
                let local = local_name(&start)?;
                if metadata.is_some() && path.is_empty() {
                    return Err(Error::Xml("multiple XML root elements".into()));
                }
                if metadata.is_none() {
                    let namespace = namespace.ok_or(Error::NotGaeb)?;
                    let namespace = str::from_utf8(namespace)
                        .map_err(|error| Error::Xml(format!("non-UTF-8 namespace: {error}")))?;
                    if local != "GAEB" || !is_supported_gaeb_namespace(namespace) {
                        return Err(Error::NotGaeb);
                    }
                    root_namespace = Some(namespace.as_bytes().to_vec());
                    metadata = Some(Metadata::new(namespace.to_owned()));
                    root_closed = true;
                }
                if direct_quantity(&path) {
                    if let Some(item) = current_item.as_mut() {
                        item.block_quantity_edit();
                    }
                }
                let gaeb = namespace == root_namespace.as_deref();
                if gaeb && local == "Item" {
                    if current_item.is_some() {
                        return Err(Error::Xml(
                            "nested GAEB Item elements are unsupported".into(),
                        ));
                    }
                    let builder = ItemBuilder::new(&start)?;
                    let (item, edit) = builder.finish(&categories);
                    items.push(item);
                    quantity_edits.push(edit);
                } else if gaeb && local == "Qty" && direct_parent_is_item(&path) {
                    if let Some(item) = current_item.as_mut() {
                        item.start_quantity();
                    }
                }
            }
            Ok((_, Event::Text(text))) => {
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
                let decoded = unescape(raw)
                    .map_err(|error| Error::Xml(format!("invalid XML entity: {error}")))?;
                let end = reader.buffer_position() as usize + source_offset;
                let range = end.checked_sub(text.as_ref().len()).map(|start| start..end);
                capture_text(
                    &path,
                    decoded.as_ref(),
                    range,
                    metadata.as_mut().expect("root established before text"),
                    &mut categories,
                    current_item.as_mut(),
                );
            }
            Ok((_, Event::CData(text))) => {
                if path.is_empty() {
                    return Err(if metadata.is_none() {
                        Error::NotGaeb
                    } else {
                        Error::Xml("CDATA outside the XML root".into())
                    });
                }
                let value = str::from_utf8(text.as_ref())
                    .map_err(|error| Error::Xml(format!("non-UTF-8 CDATA: {error}")))?;
                let event_end = reader.buffer_position() as usize + source_offset;
                let range = event_end.checked_sub(3).and_then(|content_end| {
                    content_end
                        .checked_sub(text.as_ref().len())
                        .map(|start| start..content_end)
                });
                capture_text(
                    &path,
                    value,
                    range,
                    metadata.as_mut().expect("root established before CDATA"),
                    &mut categories,
                    current_item.as_mut(),
                );
            }
            Ok((resolved, Event::End(end))) => {
                resolved_namespace(resolved)?;
                let local = str::from_utf8(end.local_name().as_ref())
                    .map_err(|error| Error::Xml(format!("non-UTF-8 element name: {error}")))?
                    .to_owned();
                let gaeb = path.last().is_some_and(|element| element.gaeb);
                if gaeb && local == "Item" {
                    if let Some(builder) = current_item.take() {
                        let (item, edit) = builder.finish(&categories);
                        items.push(item);
                        quantity_edits.push(edit);
                    }
                } else if gaeb && local == "BoQCtgy" {
                    categories.pop();
                }
                path.pop();
                if path.is_empty() {
                    root_closed = true;
                }
            }
            Ok((_, Event::DocType(_))) => {
                return Err(Error::Xml(
                    "DOCTYPE declarations are not supported in GAEB documents".into(),
                ));
            }
            Ok((_, Event::Comment(_) | Event::PI(_))) => {
                if direct_quantity(&path) {
                    if let Some(item) = current_item.as_mut() {
                        item.block_quantity_edit();
                    }
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
            Ok(_) => {}
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

fn resolved_namespace<'a>(resolved: ResolveResult<'a>) -> Result<Option<&'a [u8]>, Error> {
    match resolved {
        ResolveResult::Unbound => Ok(None),
        ResolveResult::Bound(namespace) => Ok(Some(namespace.into_inner())),
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
            return unescape(raw)
                .map(|value| Some(value.into_owned()))
                .map_err(|error| Error::Xml(format!("invalid attribute entity: {error}")));
        }
    }
    Ok(None)
}

fn direct_parent_is_item(path: &[PathElement]) -> bool {
    path.last().is_some_and(|element| element.is_gaeb("Item"))
}

fn direct_item_child(path: &[PathElement]) -> bool {
    path.len() >= 2 && path[path.len() - 2].is_gaeb("Item")
}

fn direct_quantity(path: &[PathElement]) -> bool {
    path.len() >= 2 && path[path.len() - 2].is_gaeb("Item") && path[path.len() - 1].is_gaeb("Qty")
}

fn capture_text(
    path: &[PathElement],
    raw_value: &str,
    range: Option<Range<usize>>,
    metadata: &mut Metadata,
    categories: &mut [CategoryBuilder],
    item: Option<&mut ItemBuilder>,
) {
    let Some(current) = path.last() else {
        return;
    };
    if !current.gaeb {
        return;
    }

    if let Some(item) = item {
        if direct_item_child(path) {
            match current.local.as_str() {
                "Qty" => item.capture_quantity(raw_value, range),
                "QU" => append_optional_raw(&mut item.unit, raw_value),
                "UP" => append_optional_raw(&mut item.unit_price, raw_value),
                "IT" => append_optional_raw(&mut item.total_price, raw_value),
                _ => {}
            }
        }
        if path.iter().any(|element| element.is_gaeb("Description")) {
            append_words(&mut item.description, raw_value);
        }
        return;
    }

    let value = raw_value.trim();
    if value.is_empty() {
        return;
    }
    if current.local == "LblTx" {
        if let Some(category) = categories.last_mut() {
            append_optional_words(&mut category.label, value);
        }
    }

    let in_header = path.iter().any(|element| element.is_gaeb("GAEBInfo"));
    match current.local.as_str() {
        "Version" if in_header => append_optional_raw(&mut metadata.version_text, raw_value),
        "VersDate" if in_header => append_optional_raw(&mut metadata.version_date, raw_value),
        "Date" if in_header => append_optional_raw(&mut metadata.date, raw_value),
        "Time" if in_header => append_optional_raw(&mut metadata.time, raw_value),
        "ProgSystem" if in_header => append_optional_raw(&mut metadata.program_system, raw_value),
        "ProgName" if in_header => append_optional_raw(&mut metadata.program_name, raw_value),
        "DP" => append_optional_raw(&mut metadata.phase_code, raw_value),
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

fn is_supported_gaeb_namespace(namespace: &str) -> bool {
    if namespace == "http://www.gaeb.de/GAEB_DA_XML/200407" {
        return true;
    }
    let Some(rest) = namespace.strip_prefix("http://www.gaeb.de/GAEB_DA_XML/DA") else {
        return false;
    };
    let Some((phase, version)) = rest.split_once('/') else {
        return false;
    };
    GaebVersion::from_text(version).is_some()
        && (ExchangePhase::from_code(phase).is_some() || matches!(phase, "50" | "51"))
}

fn finalize_detection(metadata: &mut Metadata, diagnostics: &mut Vec<Diagnostic>) {
    let (namespace_version, namespace_phase) = namespace_evidence(&metadata.namespace);
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
    if let (Some(namespace), Some(declared)) = (namespace_phase, declared_phase) {
        if namespace != declared {
            diagnostics.push(Diagnostic::new(
                DiagnosticKind::PhaseMismatch,
                format!("namespace identifies phase {namespace}, but <DP> declares {declared}"),
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

fn namespace_evidence(namespace: &str) -> (Option<GaebVersion>, Option<ExchangePhase>) {
    if namespace == "http://www.gaeb.de/GAEB_DA_XML/200407" {
        return (Some(GaebVersion::V3_1), None);
    }
    let Some(rest) = namespace.strip_prefix("http://www.gaeb.de/GAEB_DA_XML/DA") else {
        return (None, None);
    };
    let Some((phase, version)) = rest.split_once('/') else {
        return (None, None);
    };
    (
        GaebVersion::from_text(version),
        ExchangePhase::from_code(phase),
    )
}
