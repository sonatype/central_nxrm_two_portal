// Copyright (c) 2024-present Sonatype, Inc. All rights reserved.
// "Sonatype" is a trademark of Sonatype, Inc.

use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PublishingType {
    /// A successful upload results in a validated bundle, which must be manually published
    UserManaged,

    /// A successful upload results in a validated and automatically published deployment
    Automatic,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_representation() {
        assert_eq!(
            r#""USER_MANAGED""#,
            serde_json::to_string(&PublishingType::UserManaged)
                .expect("Failed to convert to a string")
        );
        assert_eq!(
            r#""AUTOMATIC""#,
            serde_json::to_string(&PublishingType::Automatic)
                .expect("Failed to convert to a string")
        );
    }
}
