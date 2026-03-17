use super::error::TowlConfigError;
use serde::{Deserialize, Serialize};
use std::fmt;

pub const MAX_CONFIG_STRING_LENGTH: usize = 512;

macro_rules! validated_newtype {
    ($name:ident, $field:expr, $default:expr) => {
        #[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
        pub struct $name(String);

        impl $name {
            /// # Errors
            /// Returns `TowlConfigError::ConfigValueTooLong` if value exceeds `MAX_CONFIG_STRING_LENGTH`.
            pub fn try_new(s: impl Into<String>) -> Result<Self, TowlConfigError> {
                let value = s.into();
                if value.len() > MAX_CONFIG_STRING_LENGTH {
                    return Err(TowlConfigError::ConfigValueTooLong {
                        field: $field.to_string(), // clone: format string needed for error variant
                        length: value.len(),
                        max_length: MAX_CONFIG_STRING_LENGTH,
                    });
                }
                Ok(Self(value))
            }

            /// Constructs without length validation. Only for known-safe values (defaults, test data).
            #[cfg(test)]
            pub(crate) fn new_unchecked(s: impl Into<String>) -> Self {
                Self(s.into())
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::try_new($default).expect("default value is within length limits")
            }
        }
    };
}

validated_newtype!(Owner, "github.owner", "no owner");
validated_newtype!(Repo, "github.repo", "no repo");

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_owner_rejects_oversized(s in ".{513,600}") {
            prop_assert!(Owner::try_new(s).is_err());
        }

        #[test]
        fn prop_repo_rejects_oversized(s in ".{513,600}") {
            prop_assert!(Repo::try_new(s).is_err());
        }

        #[test]
        fn prop_owner_accepts_valid(s in "[a-zA-Z0-9 _.-]{0,512}") {
            let result = Owner::try_new(&s);
            prop_assert!(result.is_ok());
            prop_assert_eq!(result.unwrap().to_string(), s);
        }

        #[test]
        fn prop_repo_accepts_valid(s in "[a-zA-Z0-9 _.-]{0,512}") {
            let result = Repo::try_new(&s);
            prop_assert!(result.is_ok());
            prop_assert_eq!(result.unwrap().to_string(), s);
        }
    }
}
