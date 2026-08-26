# openbim-gaeb-bindings

Opt-in generated typed GAEB bindings. This crate is intentionally separate from
`openbim-gaeb` so lossless parsing and XSD validation do not pay generated-code
compile cost.

Only support-matrix rows with a non-empty `typed_module` are accepted. Official
GAEB schema and fixture bytes are not included; tests read a caller-provided
fixture root from `GAEB_OFFICIAL_FIXTURES` and schema root from
`GAEB_OFFICIAL_SCHEMA_ROOT`.

GAEB decimal fields use an exact decimal wrapper rather than binary floating
point. Generated source inputs, hashes, generator configuration, reviewed
post-generation corrections, and executable evidence are recorded in
[generated-binding provenance](https://github.com/openbimrs/gaeb/blob/master/docs/generated-bindings.md).

Publication is disabled while `openbim-gaeb` depends on an immutable Git
revision of `xsd-schema`; source/workspace/package verification patches the
same exact revision into extracted candidates.
