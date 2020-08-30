#[cfg(test)]
mod tests {

    use tempfile::NamedTempFile;
    use actix_validated_forms_derive::FromMultipart;
    use actix_validated_forms::multipart::{MultipartForm, MultipartField, MultipartText};
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
    }

    #[test]
    fn it_works() {
        let mut multipart = MultipartForm::new();
        multipart.push(MultipartField::Text(MultipartText{ name: "int".to_string(), text: "5".to_string() }));
        let result = Two::try_from(multipart);
        println!("{:?}", result);
    }

}

