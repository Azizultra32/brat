use chrono::Utc;
use rand::Rng;

/// Generate a convoy ID: c-YYYYMMDD-<4hex>
pub fn generate_convoy_id() -> String {
    let date = Utc::now().format("%Y%m%d");
    let hex: u16 = rand::thread_rng().gen();
    format!("c-{}-{:04x}", date, hex)
}

/// Generate a task ID: t-YYYYMMDD-<4hex>
pub fn generate_task_id() -> String {
    let date = Utc::now().format("%Y%m%d");
    let hex: u16 = rand::thread_rng().gen();
    format!("t-{}-{:04x}", date, hex)
}

/// Generate a session ID: s-YYYYMMDD-<4hex>
pub fn generate_session_id() -> String {
    let date = Utc::now().format("%Y%m%d");
    let hex: u16 = rand::thread_rng().gen();
    format!("s-{}-{:04x}", date, hex)
}

/// Parse a convoy ID, returning (date_str, hex_suffix) if valid.
pub fn parse_convoy_id(id: &str) -> Option<(&str, &str)> {
    let parts: Vec<&str> = id.splitn(3, '-').collect();
    if parts.len() == 3 && parts[0] == "c" && parts[1].len() == 8 && parts[2].len() == 4 {
        Some((parts[1], parts[2]))
    } else {
        None
    }
}

/// Parse a task ID, returning (date_str, hex_suffix) if valid.
pub fn parse_task_id(id: &str) -> Option<(&str, &str)> {
    let parts: Vec<&str> = id.splitn(3, '-').collect();
    if parts.len() == 3 && parts[0] == "t" && parts[1].len() == 8 && parts[2].len() == 4 {
        Some((parts[1], parts[2]))
    } else {
        None
    }
}

/// Parse a session ID, returning (date_str, hex_suffix) if valid.
pub fn parse_session_id(id: &str) -> Option<(&str, &str)> {
    let parts: Vec<&str> = id.splitn(3, '-').collect();
    if parts.len() == 3 && parts[0] == "s" && parts[1].len() == 8 && parts[2].len() == 4 {
        Some((parts[1], parts[2]))
    } else {
        None
    }
}

/// Check if a string is a valid convoy ID.
pub fn is_valid_convoy_id(id: &str) -> bool {
    parse_convoy_id(id).is_some()
}

/// Check if a string is a valid task ID.
pub fn is_valid_task_id(id: &str) -> bool {
    parse_task_id(id).is_some()
}

/// Check if a string is a valid session ID.
pub fn is_valid_session_id(id: &str) -> bool {
    parse_session_id(id).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_convoy_id_format() {
        let id = generate_convoy_id();
        assert!(id.starts_with("c-"));
        assert_eq!(id.len(), 15); // c-YYYYMMDD-XXXX (1+1+8+1+4=15)
        assert!(is_valid_convoy_id(&id));
    }

    #[test]
    fn test_generate_task_id_format() {
        let id = generate_task_id();
        assert!(id.starts_with("t-"));
        assert_eq!(id.len(), 15); // t-YYYYMMDD-XXXX (1+1+8+1+4=15)
        assert!(is_valid_task_id(&id));
    }

    #[test]
    fn test_generate_session_id_format() {
        let id = generate_session_id();
        assert!(id.starts_with("s-"));
        assert_eq!(id.len(), 15); // s-YYYYMMDD-XXXX (1+1+8+1+4=15)
        assert!(is_valid_session_id(&id));
    }

    #[test]
    fn test_parse_convoy_id() {
        let (date, hex) = parse_convoy_id("c-20250116-a2f9").unwrap();
        assert_eq!(date, "20250116");
        assert_eq!(hex, "a2f9");
    }

    #[test]
    fn test_parse_task_id() {
        let (date, hex) = parse_task_id("t-20250116-3a2c").unwrap();
        assert_eq!(date, "20250116");
        assert_eq!(hex, "3a2c");
    }

    #[test]
    fn test_invalid_ids() {
        assert!(parse_convoy_id("invalid").is_none());
        assert!(parse_convoy_id("t-20250116-a2f9").is_none()); // wrong prefix
        assert!(parse_convoy_id("c-2025011-a2f9").is_none()); // short date
        assert!(parse_convoy_id("c-20250116-a2f").is_none()); // short hex
    }

    #[test]
    fn test_uniqueness() {
        let ids: Vec<String> = (0..100).map(|_| generate_convoy_id()).collect();
        let unique: std::collections::HashSet<_> = ids.iter().collect();
        // Should have mostly unique IDs (date is same, but hex should differ)
        assert!(unique.len() > 90);
    }
}
