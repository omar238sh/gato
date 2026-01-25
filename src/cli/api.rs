use std::{fs, path::PathBuf};

use crate::{get_store_path, init::create_file_layout};

pub fn init(path: PathBuf) {
    let id = fs::read_to_string(path.join(".gato"));
    match id {
        Ok(_) => {
            println!("Repo already initialized");
        }
        Err(_) => match create_file_layout(get_store_path().clone(), path) {
            Ok(()) => println!("initialized successfuly"),
            Err(_) => todo!(),
        },
    }
}
