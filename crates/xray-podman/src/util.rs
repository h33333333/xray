/// Error-friendly optional string that implements [std::fmt::Display].
#[derive(Debug)]
pub struct OptionalString(String);

impl OptionalString {
    pub fn new(val: String) -> Self {
        OptionalString(val)
    }
}

impl std::fmt::Display for OptionalString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.is_empty() {
            return write!(f, "<None>");
        }
        write!(f, "{}", self.0)
    }
}
