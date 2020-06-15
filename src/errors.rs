use error_chain::error_chain;

impl From<Box<dyn std::error::Error>> for Error {
    fn from(e: Box<dyn std::error::Error>) -> Self {
        Self::from(format!("{:?}", e))
    }
}

impl From<Box<dyn std::error::Error + Sync + Send>> for Error {
    fn from(e: Box<dyn std::error::Error + Sync + Send>) -> Self {
        Self::from(format!("{:?}", e))
    }
}

#[cfg(test)]
impl PartialEq for Error {
    fn eq(&self, _other: &Self) -> bool {
        // This might be Ok since we try to compare Result in tests
        false
    }
}

error_chain! {
    foreign_links {
        UTF8(std::string::FromUtf8Error);
        Tungtentie(tokio_tungstenite::tungstenite::error::Error);
        LibABIEOS(libabieos_sys::errors::Error);
        SerdeJson(serde_json::error::Error);
    }
    errors {
        ExpectedABI{
            description("expected shipper ABI")
            display("expected shipper ABI")
        }
    }
}
