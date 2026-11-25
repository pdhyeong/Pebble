use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Settings {
    pub device_name: String,
    pub shared_folder: PathBuf,
    pub shared_folder_display_name: String,
    pub show_folder_name: bool,
}

impl Settings {
    pub fn with_home(home: Option<PathBuf>) -> Self {
        let shared_folder = home
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Pebble")
            .join("Shared");
        Self {
            device_name: "내 기기".to_string(),
            shared_folder,
            shared_folder_display_name: "Shared".to_string(),
            show_folder_name: true,
        }
    }

    pub fn advertised_folder_name(&self) -> Option<String> {
        self.show_folder_name
            .then(|| self.shared_folder_display_name.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_folder_is_under_home() {
        let settings = Settings::with_home(Some(PathBuf::from("/home/u")));
        assert_eq!(
            settings.shared_folder,
            PathBuf::from("/home/u/Pebble/Shared")
        );
    }

    #[test]
    fn folder_name_hidden_when_disabled() {
        let mut settings = Settings::with_home(None);
        assert_eq!(settings.advertised_folder_name().as_deref(), Some("Shared"));
        settings.show_folder_name = false;
        assert_eq!(settings.advertised_folder_name(), None);
    }
}
