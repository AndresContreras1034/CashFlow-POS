use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LicenseKind {
    Purchase,
    Rental,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LicensePayload {
    pub schema_version: u8,
    pub license_id: String,
    pub installation_id: String,
    pub licensee: String,
    pub kind: LicenseKind,
    pub issued_at: String,
    pub expires_at: Option<String>,
    pub source_code_access: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SignedLicense {
    pub payload: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LicenseStatusKind {
    Unlicensed,
    Active,
    Expired,
    Invalid,
}

#[derive(Debug, Clone, Serialize)]
pub struct LicenseStatusDto {
    pub installation_id: String,
    pub status: LicenseStatusKind,
    pub license_id: Option<String>,
    pub licensee: Option<String>,
    pub kind: Option<LicenseKind>,
    pub expires_at: Option<String>,
    pub source_code_access: bool,
    pub message: String,
}
