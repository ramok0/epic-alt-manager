use std::path::PathBuf;

use crate::egl::RememberMeEntry;



#[derive(Clone, Debug, PartialEq)]
struct Legendary {
    path:PathBuf
}

impl Legendary {
    pub fn new(path:PathBuf) -> Self {
        Self {
            path
        }
    }

    pub fn get_remember_me_data() -> RememberMeEntry {
        todo!()
    }
}