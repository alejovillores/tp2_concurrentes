use std::{
    fs::File,
    io::{BufReader, Read},
};

pub struct FileReader {}

impl FileReader {
    pub fn read(file_name: &String) -> Result<String, String> {
        match File::open(file_name) {
            Ok(file) => {
                let mut buf_reader = BufReader::new(file);
                let mut contents = String::new();
                let result = buf_reader.read_to_string(&mut contents);
                match result {
                    Ok(_) => Ok(contents),
                    Err(msg) => Err(msg.to_string()),
                }
            }
            Err(error) => Err(error.to_string()),
        }
    }
}

#[cfg(test)]
mod file_reader_test {
    use super::FileReader;

    #[test]
    fn test01_when_opening_a_file_with_one_order_should_return_ok() {
        let file_name = String::from("files/test_files/one_order.json");
        let result = FileReader::read(&file_name);
        assert!(result.is_ok());
    }

    #[test]
    fn test02_when_opening_a_non_existing_file_should_return_only_error() {
        let file_name = String::from("files/test_order_files/non_existing_file.json");
        let result = FileReader::read(&file_name);

        assert!(result.is_err());
    }
}
