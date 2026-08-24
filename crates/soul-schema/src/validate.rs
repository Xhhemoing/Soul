//! Draft 2020-12 validation for the frozen contracts.
//!
//! The schema documents live in `docs/schemas/` and reference each other by
//! absolute `https://soul.local/schemas/...` URI. `soul.local` is a naming
//! authority, not a host: [`LocalRetriever`] answers every reference from text
//! embedded at compile time, so validation never touches the filesystem and
//! never opens a socket. The `resolve-http` feature of `jsonschema` is off.

use std::collections::BTreeMap;
use std::fmt;

use jsonschema::{Draft, Retrieve, Uri, Validator};
use serde_json::Value;

/// Naming authority for every Soul schema `$id`.
pub const SCHEMA_BASE: &str = "https://soul.local/schemas/";

macro_rules! schema_ids {
    ($( $variant:ident => $file:literal ),+ $(,)?) => {
        /// One variant per document in `docs/schemas/`.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum SchemaId {
            $( $variant ),+
        }

        impl SchemaId {
            /// Every schema, in a stable order. Also the order used by the
            /// freeze file `docs/schemas/schemas.lock.json`.
            pub const ALL: &'static [SchemaId] = &[ $( SchemaId::$variant ),+ ];

            /// File name relative to `docs/schemas/`.
            pub const fn file_name(self) -> &'static str {
                match self {
                    $( SchemaId::$variant => $file ),+
                }
            }

            /// The document text, embedded at compile time.
            pub const fn source(self) -> &'static str {
                match self {
                    $( SchemaId::$variant => include_str!(
                        concat!("../../../docs/schemas/", $file)
                    ) ),+
                }
            }
        }
    };
}

schema_ids! {
    Defs => "_defs.schema.json",
    Event => "event.schema.json",
    Evidence => "evidence.schema.json",
    Inference => "inference.schema.json",
    Profile => "profile.schema.json",
    Memory => "memory.schema.json",
    Contact => "contact.schema.json",
    Relationship => "relationship.schema.json",
    Audit => "audit.schema.json",
    ExportManifest => "export-manifest.schema.json",
    SoulImportV1 => "soul-import-v1.schema.json",
}

impl SchemaId {
    /// The `$id` this document declares.
    pub fn uri(self) -> String {
        format!("{SCHEMA_BASE}{}", self.file_name())
    }

    /// Look a schema up by file name.
    pub fn from_file_name(name: &str) -> Option<SchemaId> {
        SchemaId::ALL
            .iter()
            .copied()
            .find(|id| id.file_name() == name)
    }

    /// The nine documents that must draw their shared vocabulary from
    /// `_defs.schema.json` rather than inlining a private copy.
    pub const REFERENCING_DEFS: &'static [SchemaId] = &[
        SchemaId::Event,
        SchemaId::Evidence,
        SchemaId::Inference,
        SchemaId::Profile,
        SchemaId::Memory,
        SchemaId::Contact,
        SchemaId::Relationship,
        SchemaId::Audit,
        SchemaId::ExportManifest,
    ];
}

impl fmt::Display for SchemaId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.file_name())
    }
}

/// Resolves `https://soul.local/schemas/*` from embedded text.
///
/// Any other URI is an error: a contract that reaches outside the frozen set
/// should fail loudly at compile-the-validator time, not silently succeed.
#[derive(Debug, Default, Clone, Copy)]
pub struct LocalRetriever;

impl Retrieve for LocalRetriever {
    fn retrieve(
        &self,
        uri: &Uri<&str>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let raw = uri.as_str();
        let file = raw
            .strip_prefix(SCHEMA_BASE)
            .ok_or_else(|| format!("refusing to resolve non-local schema reference: {raw}"))?;
        let id = SchemaId::from_file_name(file)
            .ok_or_else(|| format!("unknown Soul schema: {file}"))?;
        Ok(serde_json::from_str(id.source())?)
    }
}

/// A validation attempt that failed, rendered as human-readable messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationFailure {
    pub schema: SchemaId,
    pub errors: Vec<String>,
}

impl fmt::Display for ValidationFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} rejected the instance:", self.schema)?;
        for e in &self.errors {
            write!(f, "\n  - {e}")?;
        }
        Ok(())
    }
}

impl std::error::Error for ValidationFailure {}

/// Failure to build the validator set at all, which means the frozen schema
/// documents are themselves broken.
#[derive(Debug, thiserror::Error)]
pub enum SchemaSetError {
    #[error("{schema} is not valid JSON: {source}")]
    Parse {
        schema: SchemaId,
        #[source]
        source: serde_json::Error,
    },
    #[error("{schema} could not be compiled as draft 2020-12: {message}")]
    Compile { schema: SchemaId, message: String },
}

/// Compiled validators for the whole frozen contract set.
#[derive(Debug)]
pub struct SchemaSet {
    validators: BTreeMap<SchemaId, Validator>,
}

impl SchemaSet {
    /// Compile every document. Cross-file `$ref`s are resolved eagerly here,
    /// so a dangling reference surfaces as an error rather than at use time.
    pub fn load() -> Result<Self, SchemaSetError> {
        let mut validators = BTreeMap::new();
        for &id in SchemaId::ALL {
            validators.insert(id, compile(id)?);
        }
        Ok(SchemaSet { validators })
    }

    pub fn validator(&self, id: SchemaId) -> &Validator {
        self.validators
            .get(&id)
            .expect("SchemaSet::load compiles every SchemaId")
    }

    pub fn is_valid(&self, id: SchemaId, instance: &Value) -> bool {
        self.validator(id).is_valid(instance)
    }

    /// Validate and collect every error, not just the first.
    pub fn validate(&self, id: SchemaId, instance: &Value) -> Result<(), ValidationFailure> {
        let errors: Vec<String> = self
            .validator(id)
            .iter_errors(instance)
            .map(|e| format!("{} at {}", e, e.instance_path))
            .collect();
        if errors.is_empty() {
            Ok(())
        } else {
            Err(ValidationFailure { schema: id, errors })
        }
    }

    /// Serialize a model and validate the result against its own contract.
    pub fn validate_model<T: serde::Serialize>(
        &self,
        id: SchemaId,
        model: &T,
    ) -> Result<Value, ValidationFailure> {
        let value = serde_json::to_value(model).map_err(|e| ValidationFailure {
            schema: id,
            errors: vec![format!("model failed to serialize: {e}")],
        })?;
        self.validate(id, &value)?;
        Ok(value)
    }
}

fn compile(id: SchemaId) -> Result<Validator, SchemaSetError> {
    let document: Value =
        serde_json::from_str(id.source()).map_err(|source| SchemaSetError::Parse {
            schema: id,
            source,
        })?;
    jsonschema::options()
        .with_draft(Draft::Draft202012)
        .with_retriever(LocalRetriever)
        .should_validate_formats(true)
        .build(&document)
        .map_err(|e| SchemaSetError::Compile {
            schema: id,
            message: e.to_string(),
        })
}

/// Parse a schema document into JSON without compiling it.
pub fn schema_document(id: SchemaId) -> Value {
    serde_json::from_str(id.source()).expect("embedded schema documents are valid JSON")
}
