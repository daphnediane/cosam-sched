/// Determines what data is included in JSON export
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonExportMode {
    /// Public-facing JSON with staff/hidden information filtered out
    Public,
    /// Staff/internal JSON with all information included
    Staff,
}
