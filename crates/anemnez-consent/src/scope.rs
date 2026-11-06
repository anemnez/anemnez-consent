//! What an answer covers, and how specific it is.
//!
//! The person's four path scopes (User, 2026-11-06): `project` (any directory the person's
//! project roots name), `cwd` (the subject's current working directory and below), `home`, and
//! `system` (everything). An action matches every scope that contains its path, and the DEEPEST
//! containing root wins, so a cwd inside a project beats the project, and both beat home and
//! system. A command has its own scope, an argv prefix, where a longer prefix wins.
//!
//! The host says where the places are ([`Places`]). The project roots are a published fact per
//! host, written only by the person's act. A cwd counts only when its place is proven: an outside
//! caller's claimed cwd matches nothing (IRIS.md Q6), so it gets no narrower than home.

/// An absolute, canonical path as components (`/tmp/a` is `["tmp", "a"]`; the root is `[]`).
/// The host canonicalizes. Matching is component-wise, never by string, so `/tmp/beck` never
/// covers `/tmp/becky`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CanonPath(Vec<String>);

/// Why a path was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathError(pub String);

impl CanonPath {
    /// A canonical absolute path from its `/`-joined form; refuses `.`, `..`, empty parts, NUL.
    ///
    /// # Errors
    /// The path is relative, or a component is not canonical.
    pub fn parse(path: &str) -> Result<Self, PathError> {
        let Some(rest) = path.strip_prefix('/') else {
            return Err(PathError(format!("'{path}' is not absolute")));
        };
        if rest.is_empty() {
            return Ok(Self(Vec::new()));
        }
        let mut parts = Vec::new();
        for c in rest.split('/') {
            if c.is_empty() || c == "." || c == ".." || c.contains('\0') {
                return Err(PathError(format!(
                    "'{c}' is not a canonical path component"
                )));
            }
            parts.push(c.to_string());
        }
        Ok(Self(parts))
    }

    /// Whether `self` is `prefix` or lies under it.
    #[must_use]
    pub fn is_under(&self, prefix: &Self) -> bool {
        self.0.starts_with(&prefix.0)
    }

    /// How deep it is (the root is 0).
    #[must_use]
    pub fn depth(&self) -> usize {
        self.0.len()
    }
}

/// The subject's working directory, and whether its place is proven (by the mesh's hop record
/// or the host itself), or only claimed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cwd {
    /// The directory.
    pub path: CanonPath,
    /// Proven, not merely claimed. An unproven cwd matches nothing.
    pub proven: bool,
}

/// Where the person's places are, supplied by the host for each decision.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Places {
    /// The person's home directory, when the host knows it.
    pub home: Option<CanonPath>,
    /// The person's project roots on this host.
    pub projects: Vec<CanonPath>,
    /// The subject's working directory, when it has one.
    pub cwd: Option<Cwd>,
}

/// What a call acts on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// A file or directory.
    Path(CanonPath),
    /// A command line, as argv.
    Command(Vec<String>),
    /// Nothing narrower than the capability itself.
    None,
}

/// What a standing answer covers.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Scope {
    /// Any of the person's project roots, and everything below.
    Project,
    /// The subject's proven working directory, and everything below.
    Cwd,
    /// The person's home, and everything below.
    Home,
    /// Every command whose argv starts with these words (never empty: that is `System`).
    Command(Vec<String>),
    /// Everything: every call of the capability.
    System,
}

impl Scope {
    /// The depth of this scope's root that contains `target`, or `None` when it does not cover
    /// it. A deeper root is a more specific answer. `System` covers everything at depth 0.
    #[must_use]
    pub fn covering_depth(&self, target: &Target, places: &Places) -> Option<usize> {
        match (self, target) {
            (Self::System, _) => Some(0),
            (Self::Project, Target::Path(t)) => places
                .projects
                .iter()
                .filter(|root| t.is_under(root))
                .map(CanonPath::depth)
                .max(),
            (Self::Cwd, Target::Path(t)) => places
                .cwd
                .as_ref()
                .filter(|c| c.proven && t.is_under(&c.path))
                .map(|c| c.path.depth()),
            (Self::Home, Target::Path(t)) => places
                .home
                .as_ref()
                .filter(|h| t.is_under(h))
                .map(CanonPath::depth),
            (Self::Command(prefix), Target::Command(argv)) => {
                (!prefix.is_empty() && argv.starts_with(prefix)).then_some(prefix.len())
            }
            (Self::Project | Self::Cwd | Self::Home | Self::Command(_), _) => None,
        }
    }
}
