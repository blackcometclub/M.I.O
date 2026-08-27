use std::collections::BTreeMap;

const MAX_FAMILY_NAME_LENGTH: usize = 256;

fn normalize_family_names(names: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut families = BTreeMap::new();
    for name in names {
        let trimmed = name.trim();
        if trimmed.is_empty()
            || trimmed.chars().count() > MAX_FAMILY_NAME_LENGTH
            || trimmed.chars().any(char::is_control)
        {
            continue;
        }
        families
            .entry(trimmed.to_lowercase())
            .or_insert_with(|| trimmed.to_owned());
    }
    families.into_values().collect()
}
fn system_font_families() -> Vec<String> {
    let mut database = fontdb::Database::new();
    database.load_system_fonts();
    normalize_family_names(
        database
            .faces()
            .flat_map(|face| face.families.iter().map(|(name, _)| name.clone())),
    )
}

#[tauri::command]
pub(crate) async fn desktop_system_font_families() -> Vec<String> {
    tauri::async_runtime::spawn_blocking(system_font_families)
        .await
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::normalize_family_names;

    #[test]
    fn sorts_deduplicates_and_rejects_unsafe_names() {
        let names = vec![
            " Noto Sans ".to_owned(),
            "noto sans".to_owned(),
            "Yu Gothic".to_owned(),
            "bad\nfont".to_owned(),
            String::new(),
        ];

        assert_eq!(
            normalize_family_names(names),
            vec!["Noto Sans".to_owned(), "Yu Gothic".to_owned()]
        );
    }
}
