#[cfg(test)]
mod tests {

    use tempfile::NamedTempFile;
    use actix_validated_forms_derive::FromMultipart;
    use actix_validated_forms::multipart::MultipartForm;
    use std::convert::TryFrom;

    //#[derive(FromMultipart, Debug)]
    struct Test {
        string: String,
        optional_string: Option<String>,
        int: i32,
        int_array: Vec<i32>,
        file: NamedTempFile,
        optional_file: Option<NamedTempFile>,
        file_array: NamedTempFile,
    }

    #[derive(FromMultipart, Debug)]
    struct Two {
        int: i32,
        b: i64,
    }

    #[test]
    fn it_works() {
        let multipart = MultipartForm::new();
        let result = Two::try_from(multipart);
        println!("{:?}", result);
    }

}

