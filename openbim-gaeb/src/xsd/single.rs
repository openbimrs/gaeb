use std::{
    collections::{HashMap, HashSet},
    fs,
    io::{Cursor, Read},
    path::{Component, Path, PathBuf},
};

use quick_xml::{
    events::{BytesDecl, BytesStart, Event},
    Decoder, Reader,
};
use xsd_schema::{
    load_and_process_schema_with_options,
    validation::{
        drive_quick_xml, SchemaValidator, ValidationError, ValidationFlags, ValidationSink,
        ValidationWarning,
    },
    PipelineConfig, SchemaProcessingOptions, SchemaSet,
};

use super::{XsdLoadOptions, XsdSchemaError};
use crate::{ValidationDiagnostic, ValidationLayer, ValidationReport, ValidationSeverity};

const MAX_ATTRIBUTES_PER_ELEMENT: usize = 1_024;
const MAX_XSD_DIAGNOSTICS: usize = 4_096;
const MAX_SCHEMA_GRAPH_DEPTH: usize = 64;
const MAX_SCHEMA_DOCUMENTS: usize = 256;
const MAX_SCHEMA_GRAPH_BYTES: usize = 8 * 1024 * 1024;

struct BoundedValidationSink<'a> {
    errors: &'a mut Vec<ValidationError>,
    warnings: &'a mut Vec<ValidationWarning>,
    omitted: &'a mut usize,
}

impl ValidationSink for BoundedValidationSink<'_> {
    fn on_error(&mut self, error: ValidationError) {
        if self.errors.len() + self.warnings.len() < MAX_XSD_DIAGNOSTICS {
            self.errors.push(error);
        } else {
            *self.omitted += 1;
        }
    }

    fn on_warning(&mut self, warning: ValidationWarning) {
        if self.errors.len() + self.warnings.len() < MAX_XSD_DIAGNOSTICS {
            self.warnings.push(warning);
        } else {
            *self.omitted += 1;
        }
    }
}

/// A compiled XSD schema graph reusable across validation calls.
pub struct XsdSchema {
    schema_set: SchemaSet,
}

impl XsdSchema {
    /// Load a confined local schema graph with strict derivation checks.
    ///
    /// Directives must remain below the root schema's directory and may not use
    /// absolute paths, parent traversal, URI locations, or symbolic links. The
    /// graph is snapshotted before compilation and bounded to 256 documents,
    /// 64 directive levels, and 8 MiB.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, XsdSchemaError> {
        Self::from_file_with_options(path, XsdLoadOptions::default())
    }

    /// Load a confined, bounded schema graph with explicit processing policy.
    pub fn from_file_with_options(
        path: impl AsRef<Path>,
        options: XsdLoadOptions,
    ) -> Result<Self, XsdSchemaError> {
        let path = path.as_ref();
        let (_staged_directory, staged_path, bytes) = stage_schema_graph(path)?;
        let mut schema_set = SchemaSet::new();
        let processing_options = SchemaProcessingOptions::default()
            .with_schema_derivation_validation(options.validate_schema_derivations);
        let _stats = load_and_process_schema_with_options(
            &bytes,
            &staged_path.to_string_lossy(),
            &mut schema_set,
            Some(PipelineConfig::default()),
            processing_options,
        )
        .map_err(|error| XsdSchemaError::Load {
            path: path.to_owned(),
            message: error.to_string(),
        })?;
        if schema_set.documents.iter().any(|document| {
            document
                .includes
                .iter()
                .any(|directive| directive.resolved_doc_id.is_none())
                || document
                    .imports
                    .iter()
                    .any(|directive| directive.resolved_doc_id.is_none())
                || document
                    .redefines
                    .iter()
                    .any(|directive| directive.resolved_doc_id.is_none())
                || document
                    .overrides
                    .iter()
                    .any(|directive| directive.resolved_doc_id.is_none())
        }) {
            return Err(XsdSchemaError::Load {
                path: path.to_owned(),
                message: "one or more schema directives could not be resolved".to_owned(),
            });
        }
        Ok(Self { schema_set })
    }
}

fn schema_load_error(path: &Path, message: impl Into<String>) -> XsdSchemaError {
    XsdSchemaError::Load {
        path: path.to_owned(),
        message: message.into(),
    }
}

fn schema_locations(path: &Path, bytes: &[u8]) -> Result<Vec<String>, XsdSchemaError> {
    const XSD_NAMESPACE: &str = "http://www.w3.org/2001/XMLSchema";
    let mut reader = Reader::from_reader(Cursor::new(bytes));
    reader.config_mut().trim_text(false);
    {
        let config = reader.config_mut();
        config.check_comments = true;
    }
    let mut buffer = Vec::new();
    let mut scopes = Vec::new();
    let mut locations = Vec::new();
    loop {
        let event = reader
            .read_event_into(&mut buffer)
            .map_err(|error| schema_load_error(path, error.to_string()))?;
        match event {
            Event::Start(element) => {
                if scopes.len() >= 256 {
                    return Err(schema_load_error(path, "schema XML depth exceeds 256"));
                }
                let declarations = validate_element_names(&element, reader.decoder(), &scopes)
                    .map_err(|error| schema_load_error(path, error))?;
                collect_schema_location(
                    path,
                    &element,
                    reader.decoder(),
                    &declarations,
                    &scopes,
                    XSD_NAMESPACE,
                    &mut locations,
                )?;
                scopes.push(declarations);
            }
            Event::Empty(element) => {
                let declarations = validate_element_names(&element, reader.decoder(), &scopes)
                    .map_err(|error| schema_load_error(path, error))?;
                collect_schema_location(
                    path,
                    &element,
                    reader.decoder(),
                    &declarations,
                    &scopes,
                    XSD_NAMESPACE,
                    &mut locations,
                )?;
            }
            Event::End(_) => {
                scopes.pop();
            }
            Event::Eof => break,
            _ => {}
        }
        buffer.clear();
    }
    Ok(locations)
}

fn collect_schema_location(
    path: &Path,
    element: &BytesStart<'_>,
    decoder: Decoder,
    declarations: &HashMap<String, String>,
    scopes: &[HashMap<String, String>],
    xsd_namespace: &str,
    locations: &mut Vec<String>,
) -> Result<(), XsdSchemaError> {
    let qualified = decoder
        .decode(element.name().as_ref())
        .map_err(|error| schema_load_error(path, error.to_string()))?
        .into_owned();
    let (prefix, local) = split_qname(&qualified);
    let namespace = match prefix {
        Some(prefix) => lookup_namespace(prefix, declarations, scopes),
        None => Ok(declarations
            .get("")
            .or_else(|| scopes.iter().rev().find_map(|scope| scope.get("")))
            .cloned()
            .unwrap_or_default()),
    }
    .map_err(|error| schema_load_error(path, error))?;
    if namespace != xsd_namespace
        || !matches!(local, "include" | "import" | "redefine" | "override")
    {
        return Ok(());
    }
    for attribute in element.attributes() {
        let attribute = attribute.map_err(|error| schema_load_error(path, error.to_string()))?;
        if attribute.key.as_ref() == b"schemaLocation" {
            let value = attribute
                .decode_and_unescape_value(decoder)
                .map_err(|error| schema_load_error(path, error.to_string()))?;
            locations.push(value.into_owned());
        }
    }
    Ok(())
}

fn resolve_confined_schema_location(
    root: &Path,
    current_directory: &Path,
    location: &str,
    requested_path: &Path,
) -> Result<PathBuf, XsdSchemaError> {
    let location_path = Path::new(location);
    if location_path.is_absolute()
        || location.contains(':')
        || location_path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(schema_load_error(
            requested_path,
            format!("schema location has a non-local path component: {location}"),
        ));
    }
    let mut resolved = current_directory.to_owned();
    for component in location_path.components() {
        match component {
            Component::CurDir => continue,
            Component::ParentDir => {
                resolved.pop();
            }
            Component::Normal(part) => resolved.push(part),
            Component::Prefix(_) | Component::RootDir => {
                unreachable!("non-local components were rejected above")
            }
        }
        if !resolved.starts_with(root) {
            return Err(schema_load_error(
                requested_path,
                "schema location escaped its root during normalization",
            ));
        }
        let metadata = fs::symlink_metadata(&resolved).map_err(|source| XsdSchemaError::Read {
            path: resolved.clone(),
            source,
        })?;
        if metadata.file_type().is_symlink() {
            return Err(schema_load_error(
                requested_path,
                format!(
                    "schema location uses a symbolic link: {}",
                    resolved.display()
                ),
            ));
        }
    }
    if !resolved.starts_with(root) {
        return Err(schema_load_error(
            requested_path,
            format!(
                "schema location escapes schema root: {}",
                resolved.display()
            ),
        ));
    }
    Ok(resolved)
}

fn stage_schema_graph(
    requested_path: &Path,
) -> Result<(tempfile::TempDir, PathBuf, Vec<u8>), XsdSchemaError> {
    let root_path = requested_path
        .canonicalize()
        .map_err(|source| XsdSchemaError::Read {
            path: requested_path.to_owned(),
            source,
        })?;
    let root_directory = root_path
        .parent()
        .ok_or_else(|| schema_load_error(requested_path, "schema has no parent directory"))?
        .to_owned();
    let mut pending = vec![(root_path.clone(), 0_usize)];
    let mut visited = HashSet::new();
    let mut files = Vec::new();
    let mut total_bytes = 0_usize;

    while let Some((path, depth)) = pending.pop() {
        if depth > MAX_SCHEMA_GRAPH_DEPTH {
            return Err(schema_load_error(
                requested_path,
                format!("schema graph depth exceeds {MAX_SCHEMA_GRAPH_DEPTH}"),
            ));
        }
        if !visited.insert(path.clone()) {
            continue;
        }
        if visited.len() > MAX_SCHEMA_DOCUMENTS {
            return Err(schema_load_error(
                requested_path,
                format!("schema graph document count exceeds {MAX_SCHEMA_DOCUMENTS}"),
            ));
        }
        let metadata = fs::symlink_metadata(&path).map_err(|source| XsdSchemaError::Read {
            path: path.clone(),
            source,
        })?;
        if !metadata.file_type().is_file() {
            return Err(schema_load_error(
                requested_path,
                format!(
                    "schema graph member is not a regular file: {}",
                    path.display()
                ),
            ));
        }
        let remaining_bytes = MAX_SCHEMA_GRAPH_BYTES - total_bytes;
        let declared_length = usize::try_from(metadata.len()).unwrap_or(usize::MAX);
        if declared_length > remaining_bytes {
            return Err(schema_load_error(
                requested_path,
                format!("schema graph bytes exceed {MAX_SCHEMA_GRAPH_BYTES}"),
            ));
        }
        let file = fs::File::open(&path).map_err(|source| XsdSchemaError::Read {
            path: path.clone(),
            source,
        })?;
        let mut bytes = Vec::with_capacity(declared_length);
        file.take((remaining_bytes + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|source| XsdSchemaError::Read {
                path: path.clone(),
                source,
            })?;
        if bytes.len() > remaining_bytes {
            return Err(schema_load_error(
                requested_path,
                format!("schema graph bytes exceed {MAX_SCHEMA_GRAPH_BYTES}"),
            ));
        }
        total_bytes += bytes.len();
        for location in schema_locations(&path, &bytes)? {
            let resolved = resolve_confined_schema_location(
                &root_directory,
                path.parent().unwrap_or(&root_directory),
                &location,
                requested_path,
            )?;
            pending.push((resolved, depth + 1));
        }
        let relative = path
            .strip_prefix(&root_directory)
            .map_err(|_| schema_load_error(requested_path, "schema graph escaped its root"))?
            .to_owned();
        files.push((relative, bytes));
    }

    let directory = tempfile::tempdir().map_err(|source| XsdSchemaError::Read {
        path: requested_path.to_owned(),
        source,
    })?;
    for (relative, bytes) in &files {
        let staged = directory.path().join(relative);
        if let Some(parent) = staged.parent() {
            fs::create_dir_all(parent).map_err(|source| XsdSchemaError::Read {
                path: parent.to_owned(),
                source,
            })?;
        }
        fs::write(&staged, bytes).map_err(|source| XsdSchemaError::Read {
            path: staged,
            source,
        })?;
    }
    let root_relative = root_path
        .strip_prefix(&root_directory)
        .map_err(|_| schema_load_error(requested_path, "root schema escaped its directory"))?;
    let staged_root = directory.path().join(root_relative);
    let root_bytes = fs::read(&staged_root).map_err(|source| XsdSchemaError::Read {
        path: staged_root.clone(),
        source,
    })?;
    Ok((directory, staged_root, root_bytes))
}

impl XsdSchema {
    /// Validate one XML document using the compiled schema graph.
    pub fn validate(&self, xml: &[u8]) -> Result<ValidationReport, XsdSchemaError> {
        if let Err(message) = validate_xml_document_shape(xml) {
            return Ok(ValidationReport::new(vec![ValidationDiagnostic::new(
                ValidationLayer::Xsd,
                "XSD-XML-PARSE",
                ValidationSeverity::Error,
                message,
            )]));
        }
        let validator = SchemaValidator::new(&self.schema_set, ValidationFlags::default());
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut omitted = 0;
        let sink = BoundedValidationSink {
            errors: &mut errors,
            warnings: &mut warnings,
            omitted: &mut omitted,
        };
        let mut runtime = validator.start_run(sink);
        let result = drive_quick_xml(Cursor::new(xml), &mut runtime, &self.schema_set);
        drop(runtime);
        let mut diagnostics = Vec::new();
        if let Err(error) = result {
            diagnostics.push(ValidationDiagnostic::new(
                ValidationLayer::Xsd,
                "XSD-XML-PARSE",
                ValidationSeverity::Error,
                error.to_string(),
            ));
        }

        diagnostics.extend(errors.into_iter().map(|error| {
            let location = error.location.as_ref();
            ValidationDiagnostic::new(
                ValidationLayer::Xsd,
                error.constraint,
                ValidationSeverity::Error,
                error.message,
            )
            .at_line(location.map(|it| it.line), location.map(|it| it.column))
            .at_optional_location(error.element_path)
        }));
        for warning in warnings {
            let location = warning.location.as_ref();
            diagnostics.push(
                ValidationDiagnostic::new(
                    ValidationLayer::Xsd,
                    warning.code,
                    ValidationSeverity::Warning,
                    warning.message,
                )
                .at_line(location.map(|it| it.line), location.map(|it| it.column)),
            );
        }
        if omitted > 0 {
            diagnostics.push(ValidationDiagnostic::new(
                ValidationLayer::Xsd,
                "XSD-DIAGNOSTICS-TRUNCATED",
                ValidationSeverity::Error,
                format!("validation diagnostics exceeded {MAX_XSD_DIAGNOSTICS}; omitted {omitted} additional diagnostics"),
            ));
        }
        Ok(ValidationReport::new(diagnostics))
    }
}

fn validate_xml_document_shape(xml: &[u8]) -> Result<(), String> {
    const MAX_DEPTH: usize = 256;
    let mut reader = Reader::from_reader(Cursor::new(xml));
    reader.config_mut().trim_text(false);
    reader.config_mut().check_comments = true;
    let mut buffer = Vec::new();
    let mut depth = 0_usize;
    let mut saw_root = false;
    let mut root_closed = false;
    let mut saw_declaration = false;
    let mut saw_prolog_content = false;
    let mut namespace_scopes = Vec::new();

    loop {
        let event = reader
            .read_event_into(&mut buffer)
            .map_err(|error| error.to_string())?;
        match event {
            Event::Start(element) => {
                let declarations =
                    validate_element_names(&element, reader.decoder(), &namespace_scopes)?;
                if depth == 0 {
                    if saw_root || root_closed {
                        return Err("XML document contains more than one root element".to_owned());
                    }
                    saw_root = true;
                }
                if depth >= MAX_DEPTH {
                    return Err(format!("XML nesting depth exceeds {MAX_DEPTH} elements"));
                }
                namespace_scopes.push(declarations);
                depth += 1;
            }
            Event::Empty(element) => {
                validate_element_names(&element, reader.decoder(), &namespace_scopes)?;
                if depth == 0 {
                    if saw_root || root_closed {
                        return Err("XML document contains more than one root element".to_owned());
                    }
                    saw_root = true;
                    root_closed = true;
                }
            }
            Event::End(_) => {
                if depth == 0 {
                    return Err("XML document contains an unmatched closing element".to_owned());
                }
                depth -= 1;
                namespace_scopes.pop();
                if depth == 0 {
                    root_closed = true;
                }
            }
            Event::Text(text) => {
                validate_xml_content_chars(reader.decoder(), text.as_ref(), "text")?;
                if depth == 0 {
                    if !saw_root {
                        saw_prolog_content = true;
                    }
                    if !text
                        .as_ref()
                        .iter()
                        .all(|byte| matches!(byte, b' ' | b'\t' | b'\r' | b'\n'))
                    {
                        return Err(
                            "non-whitespace text appears outside the root element".to_owned()
                        );
                    }
                }
            }
            Event::CData(data) => {
                validate_xml_chars(reader.decoder(), data.as_ref(), "CDATA")?;
                if depth == 0 {
                    return Err("CDATA appears outside the root element".to_owned());
                }
            }
            Event::Decl(declaration) => {
                if saw_declaration || saw_prolog_content || saw_root || root_closed || depth != 0 {
                    return Err("XML declaration is not the first document construct".to_owned());
                }
                validate_xml_declaration(&declaration)?;
                saw_declaration = true;
            }
            Event::DocType(_) => {
                return Err("DOCTYPE declarations are not accepted for GAEB validation".to_owned());
            }
            Event::PI(instruction) => {
                validate_xml_chars(
                    reader.decoder(),
                    instruction.as_ref(),
                    "processing instruction",
                )?;
                let target = reader
                    .decoder()
                    .decode(instruction.target())
                    .map_err(|error| error.to_string())?;
                if !is_valid_pi_target(&target) {
                    return Err("processing instruction has an invalid target".to_owned());
                }
                if depth == 0 && !saw_root {
                    saw_prolog_content = true;
                }
            }
            Event::Comment(comment) => {
                validate_xml_chars(reader.decoder(), comment.as_ref(), "comment")?;
                if depth == 0 && !saw_root {
                    saw_prolog_content = true;
                }
            }
            Event::Eof => break,
        }
        buffer.clear();
    }
    if !saw_root || !root_closed || depth != 0 {
        return Err("XML document does not contain one complete root element".to_owned());
    }
    Ok(())
}

fn is_valid_pi_target(target: &str) -> bool {
    if target.eq_ignore_ascii_case("xml") {
        return false;
    }
    let mut chars = target.chars();
    chars.next().is_some_and(is_xml_name_start) && chars.all(is_xml_name_char)
}

fn is_xml_name_start(character: char) -> bool {
    matches!(
        character,
        ':' | 'A'..='Z' | '_' | 'a'..='z'
            | '\u{C0}'..='\u{D6}' | '\u{D8}'..='\u{F6}' | '\u{F8}'..='\u{2FF}'
            | '\u{370}'..='\u{37D}' | '\u{37F}'..='\u{1FFF}'
            | '\u{200C}'..='\u{200D}' | '\u{2070}'..='\u{218F}'
            | '\u{2C00}'..='\u{2FEF}' | '\u{3001}'..='\u{D7FF}'
            | '\u{F900}'..='\u{FDCF}' | '\u{FDF0}'..='\u{FFFD}'
            | '\u{10000}'..='\u{EFFFF}'
    )
}

fn is_xml_name_char(character: char) -> bool {
    is_xml_name_start(character)
        || matches!(
            character,
            '-' | '.' | '0'..='9' | '\u{B7}' | '\u{300}'..='\u{36F}' | '\u{203F}'..='\u{2040}'
        )
}

fn validate_xml_declaration(declaration: &BytesDecl<'_>) -> Result<(), String> {
    let version = declaration.version().map_err(|error| error.to_string())?;
    if version.as_ref() != b"1.0" {
        return Err("only XML 1.0 declarations are accepted".to_owned());
    }
    let declaration_text =
        std::str::from_utf8(declaration.as_ref()).map_err(|error| error.to_string())?;
    let declaration_start = BytesStart::from_content(declaration_text, 3);
    let attributes = declaration_start
        .attributes()
        .map(|attribute| attribute.map_err(|error| error.to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    if !(1..=3).contains(&attributes.len()) || attributes[0].key.as_ref() != b"version" {
        return Err("XML declaration must begin with version".to_owned());
    }
    for (index, attribute) in attributes.iter().enumerate() {
        let valid = match attribute.key.as_ref() {
            b"version" => index == 0 && attribute.value.as_ref() == b"1.0",
            b"encoding" => index == 1 && valid_encoding_name(attribute.value.as_ref()),
            b"standalone" => index > 0 && matches!(attribute.value.as_ref(), b"yes" | b"no"),
            _ => false,
        };
        if !valid {
            return Err(
                "XML declaration contains invalid or misordered pseudo-attributes".to_owned(),
            );
        }
    }
    Ok(())
}

fn valid_encoding_name(value: &[u8]) -> bool {
    let Some((first, rest)) = value.split_first() else {
        return false;
    };
    first.is_ascii_alphabetic()
        && rest
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

fn validate_xml_content_chars(decoder: Decoder, bytes: &[u8], kind: &str) -> Result<(), String> {
    let decoded = decoder.decode(bytes).map_err(|error| error.to_string())?;
    let unescaped = quick_xml::escape::unescape(&decoded).map_err(|error| error.to_string())?;
    if unescaped.chars().all(is_xml10_char) {
        Ok(())
    } else {
        Err(format!("{kind} contains a character forbidden by XML 1.0"))
    }
}

fn validate_xml_chars(decoder: Decoder, bytes: &[u8], kind: &str) -> Result<(), String> {
    let decoded = decoder.decode(bytes).map_err(|error| error.to_string())?;
    if decoded.chars().all(is_xml10_char) {
        Ok(())
    } else {
        Err(format!("{kind} contains a character forbidden by XML 1.0"))
    }
}

fn is_xml10_char(character: char) -> bool {
    matches!(
        character,
        '\u{9}' | '\u{A}' | '\u{D}' | '\u{20}'..='\u{D7FF}' | '\u{E000}'..='\u{FFFD}' | '\u{10000}'..='\u{10FFFF}'
    )
}

fn validate_element_names(
    element: &BytesStart<'_>,
    decoder: Decoder,
    scopes: &[HashMap<String, String>],
) -> Result<HashMap<String, String>, String> {
    let element_name = decoder
        .decode(element.name().as_ref())
        .map_err(|error| error.to_string())?
        .into_owned();
    if !is_valid_qname(&element_name) {
        return Err("element has an invalid XML qualified name".to_owned());
    }

    let mut attributes = Vec::new();
    for (attribute_count, attribute) in element.attributes().enumerate() {
        if attribute_count >= MAX_ATTRIBUTES_PER_ELEMENT {
            return Err(format!(
                "XML element attribute count exceeds {MAX_ATTRIBUTES_PER_ELEMENT}"
            ));
        }
        let attribute = attribute.map_err(|error| error.to_string())?;
        validate_xml_content_chars(decoder, attribute.value.as_ref(), "attribute value")?;
        let name = decoder
            .decode(attribute.key.as_ref())
            .map_err(|error| error.to_string())?
            .into_owned();
        if !is_valid_qname(&name) {
            return Err("attribute has an invalid XML qualified name".to_owned());
        }
        let value = attribute
            .decode_and_unescape_value(decoder)
            .map_err(|error| error.to_string())?
            .into_owned();
        attributes.push((name, value));
    }

    let mut declarations = HashMap::new();
    for (name, value) in &attributes {
        let prefix = if name == "xmlns" {
            Some("")
        } else {
            name.strip_prefix("xmlns:")
        };
        if let Some(prefix) = prefix {
            validate_namespace_binding(prefix, value)?;
            if declarations
                .insert(prefix.to_owned(), value.to_owned())
                .is_some()
            {
                return Err("duplicate namespace declaration".to_owned());
            }
        }
    }

    validate_bound_qname(&element_name, &declarations, scopes, "element")?;
    let mut expanded_attributes = HashSet::new();
    for (name, _) in &attributes {
        if name == "xmlns" || name.starts_with("xmlns:") {
            continue;
        }
        let (prefix, local_name) = split_qname(name);
        let namespace = prefix
            .map(|prefix| lookup_namespace(prefix, &declarations, scopes))
            .transpose()?
            .unwrap_or_default();
        if !expanded_attributes.insert((namespace, local_name.to_owned())) {
            return Err("attributes have duplicate expanded names".to_owned());
        }
    }
    Ok(declarations)
}

const XML_NAMESPACE: &str = "http://www.w3.org/XML/1998/namespace";
const XMLNS_NAMESPACE: &str = "http://www.w3.org/2000/xmlns/";

fn validate_namespace_binding(prefix: &str, namespace: &str) -> Result<(), String> {
    if prefix == "xmlns" {
        return Err("the reserved xmlns prefix cannot be declared".to_owned());
    }
    if namespace == XMLNS_NAMESPACE {
        return Err("the reserved xmlns namespace cannot be bound".to_owned());
    }
    if prefix == "xml" {
        if namespace != XML_NAMESPACE {
            return Err("the xml prefix must bind the reserved XML namespace".to_owned());
        }
    } else if namespace == XML_NAMESPACE {
        return Err("only the xml prefix may bind the reserved XML namespace".to_owned());
    }
    if !prefix.is_empty() && namespace.is_empty() {
        return Err("XML 1.0 does not permit prefix undeclaration".to_owned());
    }
    Ok(())
}

fn split_qname(name: &str) -> (Option<&str>, &str) {
    name.split_once(':')
        .map_or((None, name), |(prefix, local)| (Some(prefix), local))
}

fn lookup_namespace(
    prefix: &str,
    declarations: &HashMap<String, String>,
    scopes: &[HashMap<String, String>],
) -> Result<String, String> {
    if prefix == "xml" {
        return Ok(XML_NAMESPACE.to_owned());
    }
    declarations
        .get(prefix)
        .or_else(|| scopes.iter().rev().find_map(|scope| scope.get(prefix)))
        .cloned()
        .ok_or_else(|| format!("qualified name uses undeclared prefix {prefix}"))
}

fn validate_bound_qname(
    name: &str,
    declarations: &HashMap<String, String>,
    scopes: &[HashMap<String, String>],
    kind: &str,
) -> Result<(), String> {
    if let (Some(prefix), _) = split_qname(name) {
        lookup_namespace(prefix, declarations, scopes)
            .map(|_| ())
            .map_err(|_| format!("{kind} uses undeclared namespace prefix {prefix}"))?;
    }
    Ok(())
}

fn is_valid_qname(name: &str) -> bool {
    let mut parts = name.split(':');
    let first = parts.next().unwrap_or_default();
    let second = parts.next();
    if parts.next().is_some() {
        return false;
    }
    is_valid_ncname(first) && second.is_none_or(is_valid_ncname)
}

fn is_valid_ncname(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|character| character != ':' && is_xml_name_start(character))
        && chars.all(|character| character != ':' && is_xml_name_char(character))
}
