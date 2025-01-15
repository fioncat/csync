pub type RetCode = i32;

pub const AUTHN_ERROR: RetCode = 701;
pub const AUTHZ_ERROR: RetCode = 702;
pub const DATABASE_ERROR: RetCode = 703;
pub const TOKEN_ERROR: RetCode = 704;
pub const SECRET_ERROR: RetCode = 705;
pub const JSON_ERROR: RetCode = 706;

pub const fn get_message(code: RetCode) -> &'static str {
    match code {
        AUTHN_ERROR => "Authentication error",
        AUTHZ_ERROR => "Authorization error",
        DATABASE_ERROR => "Database error",
        TOKEN_ERROR => "Token error",
        SECRET_ERROR => "Secret error",
        JSON_ERROR => "JSON error",
        _ => "Unknown error",
    }
}
