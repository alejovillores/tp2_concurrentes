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
