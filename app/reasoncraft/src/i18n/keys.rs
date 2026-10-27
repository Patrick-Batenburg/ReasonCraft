//! How the code's keys name Fluent messages.
//!
//! Keys are dotted (`ribbon.tab.home`). Fluent identifiers cannot contain
//! dots, so each dot maps to a dash in the `.ftl` files (`ribbon-tab-home`).

/// The Fluent message id of `key`.
pub(super) fn message_id(key: &str) -> String {
    key.replace('.', "-")
}

#[cfg(test)]
mod tests {
    use super::super::bundles::EN;
    use super::super::t_lang;
    use super::*;

    #[test]
    fn dots_become_dashes() {
        assert_eq!(message_id("ribbon.tab.home"), "ribbon-tab-home");
        assert_eq!(message_id("settings.author_hint"), "settings-author_hint");
    }

    #[test]
    fn every_source_type_has_a_name_and_a_prompt() {
        use reasoncraft_core::SourceType;
        for st in SourceType::ALL {
            let key = st.key();
            for id in [format!("source-{key}"), format!("source-prompt-{key}")] {
                let line = format!("\n{id} = ");
                assert!(EN.contains(&line), "en.ftl lacks `{id}`");
            }
            for key in [format!("source.{key}"), format!("source.prompt.{key}")] {
                assert_ne!(t_lang("en", &key), key);
            }
        }
    }
}
