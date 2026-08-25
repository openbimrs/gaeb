use std::{ops::Range, str};

use quick_xml::{
    escape::unescape,
    events::{BytesStart, Event},
    Reader,
};

use crate::{
    CategoryRef, Diagnostic, DiagnosticKind, Error, ExchangePhase, GaebVersion, Item, Metadata,
};

pub(crate) struct Parsed {
    pub metadata: Metadata,
    pub diagnostics: Vec<Diagnostic>,
    pub items: Vec<Item>,
    pub quantity_ranges: Vec<Option<Range<usize>>>,
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
    quantity: Option<String>,
    quantity_range: Option<Range<usize>>,
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
            quantity: None,
            quantity_range: None,
            unit: None,
            unit_price: None,
            total_price: None,
            description: String::new(),
        })
    }

    fn finish(self, categories: &[CategoryBuilder]) -> (Item, Option<Range<usize>>) {
        let description = (!self.description.is_empty()).then_some(self.description);
        (
            Item {
                id: self.id.unwrap_or_default(),
                outline_number: self.outline_number,
                quantity: self.quantity,
                unit: self.unit,
                unit_price: self.unit_price,
                total_price: self.total_price,
                description,
                category_path: categories.iter().map(CategoryBuilder::as_ref).collect(),
            },
            self.quantity_range,
        )
    }
}

pub(crate) fn parse(source: &[u8], source_offset: usize) -> Result<Parsed, Error> {
    let mut reader = Reader::from_reader(source);
    reader.config_mut().trim_text(false);

    let mut metadata = None;
    let mut diagnostics = Vec::new();
    let mut items = Vec::new();
    let mut quantity_ranges = Vec::new();
    let mut path: Vec<String> = Vec::new();
    let mut categories: Vec<CategoryBuilder> = Vec::new();
    let mut current_item: Option<ItemBuilder> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(start)) => {
                let local = local_name(&start)?;
                if metadata.is_none() {
                    if local != "GAEB" {
                        return Err(Error::NotGaeb);
                    }
                    let namespace = root_namespace(&start)?.ok_or(Error::NotGaeb)?;
                    if !namespace.starts_with("http://www.gaeb.de/GAEB_DA_XML/") {
                        return Err(Error::NotGaeb);
                    }
                    metadata = Some(Metadata::new(namespace));
                }
                path.push(local.clone());
                if local == "BoQCtgy" {
                    categories.push(CategoryBuilder {
                        id: attribute(&start, b"ID")?,
                        outline_number: attribute(&start, b"RNoPart")?,
                        label: None,
                    });
                } else if local == "Item" {
                    if current_item.is_some() {
                        return Err(Error::Xml(
                            "nested GAEB Item elements are unsupported".into(),
                        ));
                    }
                    current_item = Some(ItemBuilder::new(&start)?);
                }
            }
            Ok(Event::Empty(start)) => {
                let local = local_name(&start)?;
                if metadata.is_none() {
                    if local != "GAEB" {
                        return Err(Error::NotGaeb);
                    }
                    let namespace = root_namespace(&start)?.ok_or(Error::NotGaeb)?;
                    if !namespace.starts_with("http://www.gaeb.de/GAEB_DA_XML/") {
                        return Err(Error::NotGaeb);
                    }
                    metadata = Some(Metadata::new(namespace));
                }
                if local == "Item" {
                    let builder = ItemBuilder::new(&start)?;
                    let (item, range) = builder.finish(&categories);
                    items.push(item);
                    quantity_ranges.push(range);
                }
            }
            Ok(Event::Text(text)) => {
                let raw = str::from_utf8(text.as_ref())
                    .map_err(|error| Error::Xml(format!("non-UTF-8 XML text: {error}")))?;
                if metadata.is_none() {
                    if raw.trim().is_empty() {
                        continue;
                    }
                    return Err(Error::NotGaeb);
                }
                let decoded = unescape(raw)
                    .map_err(|error| Error::Xml(format!("invalid XML entity: {error}")))?;
                let value = decoded.as_ref();
                let end = reader.buffer_position() as usize + source_offset;
                let range = end.checked_sub(text.as_ref().len()).map(|start| start..end);
                capture_text(
                    &path,
                    value,
                    range,
                    metadata.as_mut().expect("root established before text"),
                    &mut categories,
                    current_item.as_mut(),
                );
            }
            Ok(Event::CData(text)) => {
                if metadata.is_none() {
                    return Err(Error::NotGaeb);
                }
                let value = str::from_utf8(text.as_ref())
                    .map_err(|error| Error::Xml(format!("non-UTF-8 CDATA: {error}")))?;
                capture_text(
                    &path,
                    value,
                    None,
                    metadata.as_mut().expect("root established before CDATA"),
                    &mut categories,
                    current_item.as_mut(),
                );
            }
            Ok(Event::End(end)) => {
                let local = str::from_utf8(end.local_name().as_ref())
                    .map_err(|error| Error::Xml(format!("non-UTF-8 element name: {error}")))?
                    .to_owned();
                if local == "Item" {
                    if let Some(builder) = current_item.take() {
                        let (item, range) = builder.finish(&categories);
                        items.push(item);
                        quantity_ranges.push(range);
                    }
                } else if local == "BoQCtgy" {
                    categories.pop();
                }
                path.pop();
            }
            Ok(Event::DocType(_)) => {
                return Err(Error::Xml(
                    "DOCTYPE declarations are not supported in GAEB documents".into(),
                ));
            }
            Ok(Event::Eof) => {
                if path.is_empty() {
                    break;
                }
                return Err(Error::Xml(format!(
                    "unexpected end of file with unclosed <{}>",
                    path.last().map(String::as_str).unwrap_or("unknown")
                )));
            }
            Ok(_) => {}
            Err(error) => return Err(Error::Xml(error.to_string())),
        }
    }

    let mut metadata = metadata.ok_or(Error::NotGaeb)?;
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
        quantity_ranges,
    })
}

fn local_name(start: &BytesStart<'_>) -> Result<String, Error> {
    str::from_utf8(start.local_name().as_ref())
        .map(str::to_owned)
        .map_err(|error| Error::Xml(format!("non-UTF-8 element name: {error}")))
}

fn attribute(start: &BytesStart<'_>, key: &[u8]) -> Result<Option<String>, Error> {
    for result in start.attributes().with_checks(false) {
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

fn root_namespace(start: &BytesStart<'_>) -> Result<Option<String>, Error> {
    let qualified = start.name();
    let prefix = qualified
        .as_ref()
        .split(|byte| *byte == b':')
        .next()
        .filter(|_| qualified.as_ref().contains(&b':'));
    let key = prefix.map_or_else(
        || b"xmlns".to_vec(),
        |prefix| {
            let mut key = b"xmlns:".to_vec();
            key.extend_from_slice(prefix);
            key
        },
    );
    attribute(start, &key)
}

fn capture_text(
    path: &[String],
    raw_value: &str,
    range: Option<Range<usize>>,
    metadata: &mut Metadata,
    categories: &mut [CategoryBuilder],
    item: Option<&mut ItemBuilder>,
) {
    let value = raw_value.trim();
    if value.is_empty() {
        return;
    }
    let current = path.last().map(String::as_str).unwrap_or_default();

    if let Some(item) = item {
        let direct_item_child = path.len() >= 2 && path[path.len() - 2] == "Item";
        if direct_item_child {
            match current {
                "Qty" => {
                    item.quantity = Some(value.to_owned());
                    item.quantity_range = range;
                }
                "QU" => item.unit = Some(value.to_owned()),
                "UP" => item.unit_price = Some(value.to_owned()),
                "IT" => item.total_price = Some(value.to_owned()),
                _ => {}
            }
        }
        if path.iter().any(|element| element == "Description") {
            append_words(&mut item.description, value);
        }
        return;
    }

    if current == "LblTx" {
        if let Some(category) = categories.last_mut() {
            append_optional_words(&mut category.label, value);
        }
    }

    let in_header = path.iter().any(|element| element == "GAEBInfo");
    match current {
        "Version" if in_header && metadata.version_text.is_none() => {
            metadata.version_text = Some(value.to_owned());
        }
        "VersDate" if in_header && metadata.version_date.is_none() => {
            metadata.version_date = Some(value.to_owned());
        }
        "Date" if in_header && metadata.date.is_none() => metadata.date = Some(value.to_owned()),
        "Time" if in_header && metadata.time.is_none() => metadata.time = Some(value.to_owned()),
        "ProgSystem" if in_header && metadata.program_system.is_none() => {
            metadata.program_system = Some(value.to_owned());
        }
        "ProgName" if in_header && metadata.program_name.is_none() => {
            metadata.program_name = Some(value.to_owned());
        }
        "DP" if metadata.phase_code.is_none() => metadata.phase_code = Some(value.to_owned()),
        _ => {}
    }
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
