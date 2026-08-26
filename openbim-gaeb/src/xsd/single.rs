use std::{fs, io::Cursor, path::Path};

use quick_xml::{events::Event, Reader};
use xsd_schema::{
    load_and_process_schema_with_options,
    validation::{drive_quick_xml, CollectingValidationSink, SchemaValidator, ValidationFlags},
    PipelineConfig, SchemaProcessingOptions, SchemaSet,
};

use super::{XsdLoadOptions, XsdSchemaError};
use crate::{ValidationDiagnostic, ValidationLayer, ValidationReport, ValidationSeverity};

/// A compiled XSD schema graph reusable across validation calls.
pub struct XsdSchema {
    schema_set: SchemaSet,
}

impl XsdSchema {
    /// Load a schema and all local includes/imports with strict derivation checks.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, XsdSchemaError> {
        Self::from_file_with_options(path, XsdLoadOptions::default())
    }

    /// Load a schema graph with explicit schema-level processing policy.
    pub fn from_file_with_options(
        path: impl AsRef<Path>,
        options: XsdLoadOptions,
    ) -> Result<Self, XsdSchemaError> {
        let path = path.as_ref();
        let bytes = fs::read(path).map_err(|source| XsdSchemaError::Read {
            path: path.to_owned(),
            source,
        })?;
        let mut schema_set = SchemaSet::new();
        let processing_options = SchemaProcessingOptions::default()
            .with_schema_derivation_validation(options.validate_schema_derivations);
        let stats = load_and_process_schema_with_options(
            &bytes,
            &path.to_string_lossy(),
            &mut schema_set,
            Some(PipelineConfig::default()),
            processing_options,
        )
        .map_err(|error| XsdSchemaError::Load {
            path: path.to_owned(),
            message: error.to_string(),
        })?;
        if stats
            .directive_result
            .as_ref()
            .is_some_and(|directives| directives.error_count > 0)
        {
            return Err(XsdSchemaError::Load {
                path: path.to_owned(),
                message: "one or more schema directives could not be resolved".to_owned(),
            });
        }
        Ok(Self { schema_set })
    }
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
        let sink = CollectingValidationSink {
            errors: &mut errors,
            warnings: &mut warnings,
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
        Ok(ValidationReport::new(diagnostics))
    }
}

fn validate_xml_document_shape(xml: &[u8]) -> Result<(), String> {
    const MAX_DEPTH: usize = 256;
    let mut reader = Reader::from_reader(Cursor::new(xml));
    reader.config_mut().trim_text(false);
    let mut buffer = Vec::new();
    let mut depth = 0_usize;
    let mut saw_root = false;
    let mut root_closed = false;
    let mut saw_declaration = false;
    let mut saw_prolog_content = false;

    loop {
        let event = reader
            .read_event_into(&mut buffer)
            .map_err(|error| error.to_string())?;
        match event {
            Event::Start(_) => {
                if depth == 0 {
                    if saw_root || root_closed {
                        return Err("XML document contains more than one root element".to_owned());
                    }
                    saw_root = true;
                }
                if depth >= MAX_DEPTH {
                    return Err(format!("XML nesting depth exceeds {MAX_DEPTH} elements"));
                }
                depth += 1;
            }
            Event::Empty(_) => {
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
                if depth == 0 {
                    root_closed = true;
                }
            }
            Event::Text(text) if depth == 0 => {
                if !text
                    .as_ref()
                    .iter()
                    .all(|byte| matches!(byte, b' ' | b'\t' | b'\r' | b'\n'))
                {
                    return Err("non-whitespace text appears outside the root element".to_owned());
                }
            }
            Event::CData(_) if depth == 0 => {
                return Err("CDATA appears outside the root element".to_owned());
            }
            Event::Decl(_) => {
                if saw_declaration || saw_prolog_content || saw_root || root_closed || depth != 0 {
                    return Err("XML declaration is not the first document construct".to_owned());
                }
                saw_declaration = true;
            }
            Event::DocType(_) => {
                return Err("DOCTYPE declarations are not accepted for GAEB validation".to_owned());
            }
            Event::Comment(_) | Event::PI(_) if depth == 0 && !saw_root => {
                saw_prolog_content = true;
            }
            Event::Eof => break,
            _ => {}
        }
        buffer.clear();
    }
    if !saw_root || !root_closed || depth != 0 {
        return Err("XML document does not contain one complete root element".to_owned());
    }
    Ok(())
}
