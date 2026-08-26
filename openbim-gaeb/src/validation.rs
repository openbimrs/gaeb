/// Validation subsystem that produced a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ValidationLayer {
    /// XML Schema validation.
    Xsd,
    /// GAEB rules beyond XML Schema.
    Business,
}

/// Diagnostic severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ValidationSeverity {
    Error,
    Warning,
    Info,
}

impl ValidationSeverity {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
        }
    }
}

/// Stable, machine-readable validation result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationDiagnostic {
    layer: ValidationLayer,
    code: String,
    severity: ValidationSeverity,
    message: String,
    line: Option<usize>,
    column: Option<usize>,
    location: Option<String>,
}

impl ValidationDiagnostic {
    pub(crate) fn new(
        layer: ValidationLayer,
        code: impl Into<String>,
        severity: ValidationSeverity,
        message: impl Into<String>,
    ) -> Self {
        Self {
            layer,
            code: code.into(),
            severity,
            message: message.into(),
            line: None,
            column: None,
            location: None,
        }
    }

    pub(crate) fn at_line(mut self, line: Option<usize>, column: Option<usize>) -> Self {
        self.line = line;
        self.column = column;
        self
    }

    pub(crate) fn at_location(mut self, location: impl Into<String>) -> Self {
        self.location = Some(location.into());
        self
    }

    pub(crate) fn at_optional_location(mut self, location: Option<String>) -> Self {
        self.location = location;
        self
    }

    pub(crate) fn business(
        code: impl Into<String>,
        severity: ValidationSeverity,
        message: impl Into<String>,
    ) -> Self {
        Self::new(ValidationLayer::Business, code, severity, message)
    }

    pub const fn layer(&self) -> ValidationLayer {
        self.layer
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub const fn severity(&self) -> ValidationSeverity {
        self.severity
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub const fn line(&self) -> Option<usize> {
        self.line
    }

    pub const fn column(&self) -> Option<usize> {
        self.column
    }

    pub fn location(&self) -> Option<&str> {
        self.location.as_deref()
    }
}

/// Validation diagnostics returned without mutating the source document.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ValidationReport {
    diagnostics: Vec<ValidationDiagnostic>,
}

impl ValidationReport {
    pub(crate) fn new(diagnostics: Vec<ValidationDiagnostic>) -> Self {
        Self { diagnostics }
    }

    pub(crate) fn push(&mut self, diagnostic: ValidationDiagnostic) {
        self.diagnostics.push(diagnostic);
    }

    pub fn is_valid(&self) -> bool {
        !self
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == ValidationSeverity::Error)
    }

    pub fn diagnostics(&self) -> &[ValidationDiagnostic] {
        &self.diagnostics
    }

    pub fn has_code(&self, code: &str) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code() == code)
    }
}
