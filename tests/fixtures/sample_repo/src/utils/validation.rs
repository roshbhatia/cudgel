/// Input validation utilities
use regex::Regex;

lazy_static::lazy_static! {
    static ref EMAIL_REGEX: Regex = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
    static ref PHONE_REGEX: Regex = Regex::new(r"^\+?1?-?\.?\s?\(?(\d{3})\)?[-.\s]?(\d{3})[-.\s]?(\d{4})$").unwrap();
}

/// Validates email format
pub fn is_valid_email(email: &str) -> bool {
    EMAIL_REGEX.is_match(email)
}

/// Validates phone number format (US format)
pub fn is_valid_phone(phone: &str) -> bool {
    PHONE_REGEX.is_match(phone)
}

/// Validates that a string is not empty and has reasonable length
pub fn validate_non_empty(s: &str) -> Result<(), String> {
    if s.trim().is_empty() {
        Err("String cannot be empty".to_string())
    } else if s.len() > 1000 {
        Err("String too long (max 1000 characters)".to_string())
    } else {
        Ok(())
    }
}

/// Validates age range (0-150)
pub fn validate_age(age: u32) -> Result<(), String> {
    if age > 150 {
        Err("Age must be between 0 and 150".to_string())
    } else {
        Ok(())
    }
}