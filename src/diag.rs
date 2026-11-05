use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagKind {
    Bare,
    Syntax,
    Type,
    Scope,
    Lifetime,
    Lower,
    Bounds,
}

impl DiagKind {
    fn prefix(self) -> Option<&'static str> {
        match self {
            DiagKind::Bare => None,
            DiagKind::Syntax => Some("Syntax Error"),
            DiagKind::Type => Some("Type Error"),
            DiagKind::Scope => Some("Scope Error"),
            DiagKind::Lifetime => Some("Lifetime Error"),
            DiagKind::Lower => Some("Lower Error"),
            DiagKind::Bounds => Some("Table Bounds Error"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub site: Option<usize>,
    pub kind: DiagKind,
    pub text: String,
}

impl Diagnostic {
    pub fn bare(text: impl Into<String>) -> Self {
        Self::mint(DiagKind::Bare, text)
    }
    pub fn syntax(text: impl Into<String>) -> Self {
        Self::mint(DiagKind::Syntax, text)
    }
    pub fn ty(text: impl Into<String>) -> Self {
        Self::mint(DiagKind::Type, text)
    }
    pub fn scope(text: impl Into<String>) -> Self {
        Self::mint(DiagKind::Scope, text)
    }
    pub fn lifetime(text: impl Into<String>) -> Self {
        Self::mint(DiagKind::Lifetime, text)
    }
    pub fn lower(text: impl Into<String>) -> Self {
        Self::mint(DiagKind::Lower, text)
    }
    pub fn bounds(text: impl Into<String>) -> Self {
        Self::mint(DiagKind::Bounds, text)
    }

    fn mint(kind: DiagKind, text: impl Into<String>) -> Self {
        Diagnostic {
            site: None,
            kind,
            text: text.into(),
        }
    }

    pub fn with_site(mut self, site: Option<usize>) -> Self {
        self.site = site;
        self
    }

    pub fn render(&self) -> String {
        let mut out = String::with_capacity(self.text.len() + 32);
        if let Some(line) = self.site {
            out.push_str(&format!("line {line}: "));
        }
        if let Some(prefix) = self.kind.prefix() {
            out.push_str(prefix);
            out.push_str(": ");
        }
        out.push_str(&self.text);
        out
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render())
    }
}
