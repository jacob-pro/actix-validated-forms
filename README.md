# Actix Validated Forms

Validated forms, queries, and multiparts for the Actix Web framework, using https://github.com/Keats/validator for validation.

Includes `ValidatedQuery` and `ValidatedForm` extractors, configurable with `ValidatedQueryConfig` and `ValidatedFormConfig`.

The `multipart::load_parts()` can be used to process an `actix_multipart::Multipart` into a vector of string fields and temporary files stored on disk.

The extractor `ValidatedMultipartForm` automatically runs `load_parts()` before attempting to parse into type `T` and then validate. 
`T` must implement `TryFrom<Multiparts, Error = GetError>`, but this can be implemented automatically using `#[derive(FromMultipart)]`
