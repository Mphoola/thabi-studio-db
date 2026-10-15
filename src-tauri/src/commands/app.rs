use serde::Serialize;

/// Basic application metadata shown in the status bar and About dialog.
#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    /// Where the app keeps its local database (connections, history, saved
    /// queries), shown in the privacy section of About.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_dir: Option<String>,
}

#[tauri::command]
pub fn app_info(app: tauri::AppHandle) -> AppInfo {
    use tauri::Manager;
    let package = app.package_info();
    AppInfo {
        name: package.name.clone(),
        version: package.version.to_string(),
        data_dir: app
            .path()
            .app_data_dir()
            .ok()
            .map(|dir| dir.to_string_lossy().into_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_info_serialises_with_camel_case_keys() {
        let info = AppInfo {
            name: "Thabi Studio".into(),
            version: "0.1.0".into(),
            data_dir: None,
        };
        let json = serde_json::to_value(&info).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "name": "Thabi Studio", "version": "0.1.0" })
        );
    }
}
