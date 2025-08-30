//use zeroize::Zeroize;

pub struct AmnesiaMode {
    pub mode: String,
}

impl AmnesiaMode {
    pub fn new(mode: String) -> Self {
        AmnesiaMode { mode }
    }
}

pub struct Capabilities {
    pub capability_name: String,
}

impl Capabilities {
    pub fn new(name: String) -> Self {
        Capabilities {
            capability_name: name,
        }
    }
}

pub struct Secrets {
    pub secret: String,
}

impl Secrets {
    pub fn new(secret: String) -> Self {
        Secrets { secret }
    }
}
