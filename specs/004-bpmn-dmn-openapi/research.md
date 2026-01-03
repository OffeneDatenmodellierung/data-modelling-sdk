# Research: BPMN, DMN, and OpenAPI Support

**Feature**: 004-bpmn-dmn-openapi
**Date**: 2026-01-03
**Status**: Complete

## Research Questions

### 1. XML Parsing and Validation Library Selection

**Question**: Which Rust library should we use for parsing and validating BPMN/DMN XML files against XSD schemas?

**Decision**: Use `quick-xml` with `xsd` crate for XSD validation, or `xml-rs` with manual validation.

**Rationale**:
- `quick-xml` is faster and more memory-efficient for large XML files
- `xsd` crate provides XSD schema validation capabilities
- `xml-rs` is more mature but slower
- For BPMN/DMN files (typically <10MB), performance difference is acceptable
- Primary concern is correctness of validation, not parsing speed

**Alternatives Considered**:
- `xml-rs`: Mature, well-documented, but slower. Good for smaller files.
- `quick-xml`: Faster, better for streaming, but less mature XSD support.
- Manual validation: Too complex, error-prone.

**Final Decision**: Use `quick-xml` for parsing with `xsd` crate for validation. If `xsd` crate proves insufficient, fall back to `xml-rs` with manual XSD validation using `xmllint`-style approach or external validation.

**Implementation Note**: Since we're storing files in native format (not parsing into Rust structs), we primarily need:
1. XML well-formedness checking
2. XSD schema validation
3. Basic metadata extraction (name, namespace, version)

### 2. XSD Schema Validation Approach

**Question**: How should we validate BPMN/DMN files against XSD schemas?

**Decision**: Load XSD schemas from `schemas/` directory at runtime and validate XML files against them.

**Rationale**:
- XSD schemas will be stored in `schemas/` directory as required
- Runtime validation allows schema updates without code changes
- Matches pattern used for JSON Schema validation (OpenAPI)

**Alternatives Considered**:
- Compile-time validation: Too rigid, requires code changes for schema updates
- External validation tool: Adds dependency, platform-specific issues
- No validation: Violates requirements (FR-004, FR-005)

**Final Decision**: Use `xsd` crate or similar to load XSD from `schemas/` directory and validate XML files. Provide clear error messages with line/column numbers.

### 3. OpenAPI Schema Validation Patterns

**Question**: How should we validate OpenAPI files against OpenAPI 3.1.1 JSON Schema?

**Decision**: Use existing `jsonschema` crate (already in dependencies) to validate against OpenAPI 3.1.1 JSON Schema stored in `schemas/` directory.

**Rationale**:
- `jsonschema` crate already used for ODCS/ODCL validation
- Consistent validation approach across formats
- JSON Schema validation is well-supported in Rust

**Alternatives Considered**:
- External OpenAPI validator: Adds dependency, platform issues
- Manual validation: Too complex, error-prone
- No validation: Violates requirements (FR-006)

**Final Decision**: Load OpenAPI 3.1.1 JSON Schema from `schemas/openapi-3.1.1.json` and use `jsonschema::JSONSchema::compile()` to validate OpenAPI files.

### 4. CADS Reference Extension Approach

**Question**: How should CADS assets reference BPMN/DMN/OpenAPI models?

**Decision**: Extend existing `CADSExternalLink` structure or add new reference fields to CADS asset model.

**Rationale**:
- CADS already has `external_links` in `CADSDescription`
- Need to distinguish between generic external links and model references
- Should support cross-domain references

**Alternatives Considered**:
- New `model_references` field: Cleaner separation, but adds complexity
- Extend `external_links`: Reuses existing structure, but less type-safe
- Separate reference table: Over-engineered for current needs

**Final Decision**: Add new optional fields to CADS asset:
- `bpmn_references: Option<Vec<ModelReference>>`
- `dmn_references: Option<Vec<ModelReference>>`
- `openapi_references: Option<Vec<ModelReference>>`

Where `ModelReference` contains:
- `domain_id: Uuid` (optional, None for same domain)
- `model_name: String`
- `description: Option<String>`

This provides type safety and clear separation while maintaining flexibility.

### 5. OpenAPI to ODCS Type Mapping Strategy

**Question**: How should we map OpenAPI schema types to ODCS field types?

**Decision**: Create comprehensive mapping table with fallback strategies for unsupported types.

**Rationale**:
- OpenAPI has richer type system than ODCS
- Need to preserve as much information as possible
- Some OpenAPI types don't have direct ODCS equivalents

**Mapping Table**:
- `string` → `text` (preserve format, minLength, maxLength, pattern in quality rules)
- `integer` → `long` (preserve minimum, maximum in quality rules)
- `number` → `double` (preserve minimum, maximum in quality rules)
- `boolean` → `boolean`
- `array` → Handle via nested tables or flattening (configurable)
- `object` → Handle via nested tables or flattening (configurable)
- `null` → `text` with nullable flag
- `date`, `date-time` → `timestamp` (preserve format)
- `email`, `uri`, `uuid` → `text` with format preserved

**Alternatives Considered**:
- Strict mapping (reject unsupported types): Too restrictive
- Loose mapping (everything to text): Loses type information
- Custom ODCS extensions: Violates ODCS standard

**Final Decision**: Use mapping table above with quality rules to preserve constraints. For nested objects/arrays, create separate related tables by default, with optional flattening for simple cases.

### 6. File Naming Conflict Resolution

**Question**: How should we handle file naming conflicts when importing models?

**Decision**: Generate unique filenames by appending a counter (e.g., `model_1.bpmn.xml`, `model_2.bpmn.xml`).

**Rationale**:
- Non-destructive by default
- Predictable behavior
- Can be overridden by explicit overwrite flag in future

**Alternatives Considered**:
- Overwrite by default: Too destructive, risk of data loss
- Prompt user: Not applicable for SDK (no UI)
- Reject with error: Too restrictive, requires manual cleanup

**Final Decision**: Auto-generate unique names. Future enhancement: Add `overwrite: bool` parameter to import functions.

### 7. Cross-Domain Reference Validation

**Question**: How should we validate cross-domain references to BPMN/DMN/OpenAPI models?

**Decision**: Validate references exist when creating/updating CADS assets. Use `StorageBackend` to check file existence.

**Rationale**:
- Prevents broken references
- Can be done efficiently with async file existence checks
- Matches existing pattern for ODCS table references

**Alternatives Considered**:
- Lazy validation: Allows broken references, harder to debug
- No validation: Violates requirements (FR-011)
- Reference registry: Over-engineered, adds complexity

**Final Decision**: Validate references synchronously during CADS asset creation/update. For cross-domain references, check file existence in target domain directory.

### 8. Schema File Sources

**Question**: Where should we obtain BPMN 2.0 XSD, DMN 1.3 XSD, and OpenAPI 3.1.1 JSON Schema files?

**Decision**: Download from official sources and store in `schemas/` directory.

**Sources**:
- **BPMN 2.0 XSD**: OMG (Object Management Group) - https://www.omg.org/spec/BPMN/2.0/
- **DMN 1.3 XSD**: OMG - https://www.omg.org/spec/DMN/1.3/
- **OpenAPI 3.1.1 JSON Schema**: OpenAPI Initiative - https://github.com/OAI/OpenAPI-Specification/blob/main/schemas/v3.1/schema.json

**Rationale**:
- Official sources ensure correctness
- Schemas are versioned and stable
- Can be updated when new versions are released

**Final Decision**: Download official schemas and commit to `schemas/` directory. Document source URLs in `schemas/README.md`.

## Implementation Notes

1. **Minimal Parsing**: Since we're storing native formats, we only need to parse enough to:
   - Extract model name/ID for file naming
   - Validate against schemas
   - Extract basic metadata for references

2. **Error Messages**: Provide clear, actionable error messages with:
   - File path
   - Line/column numbers (for XML)
   - Specific validation error
   - Suggested fixes where possible

3. **Performance**: For typical file sizes (<10MB), parsing and validation should be fast enough. Consider streaming for very large files in future.

4. **WASM Considerations**: XML parsing libraries must be WASM-compatible. `quick-xml` and `xml-rs` both support WASM.

5. **Feature Flags**: Add Cargo features:
   - `bpmn` - BPMN support (default: false)
   - `dmn` - DMN support (default: false)
   - `openapi` - OpenAPI support (default: false)
   - Or combine into `process-models` feature (default: false)

## Dependencies to Add

```toml
# XML parsing and validation (feature-gated)
[features]
bpmn = ["quick-xml", "xsd"]
dmn = ["quick-xml", "xsd"]
openapi = []  # Uses existing jsonschema

[dependencies]
quick-xml = { version = "0.31", optional = true, features = ["serialize"] }
xsd = { version = "0.5", optional = true }  # Or alternative XSD validator
```

**Note**: `xsd` crate may not exist or may have different name. Research required during implementation. Alternative: Use `xmllint`-style validation or external XSD validator.

## Open Questions Resolved

All research questions have been resolved. No remaining `NEEDS CLARIFICATION` markers.
