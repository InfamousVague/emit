use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionManifest {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub category: String,
    pub bundled: bool,
    pub default_enabled: bool,
    pub has_settings: bool,
}

pub fn all_manifests() -> Vec<ExtensionManifest> {
    vec![
        ExtensionManifest {
            id: "notion".into(),
            name: "Notion".into(),
            description: "Search Notion pages, databases, and tasks".into(),
            icon: "NotePencil".into(),
            category: "Productivity".into(),
            bundled: true,
            default_enabled: false,
            has_settings: true,
        },
        ExtensionManifest {
            id: "color-picker".into(),
            name: "Color Picker".into(),
            description: "Pick colors from your screen and save palettes".into(),
            icon: "Eyedropper".into(),
            category: "Utilities".into(),
            bundled: true,
            default_enabled: true,
            has_settings: false,
        },
        ExtensionManifest {
            id: "password-generator".into(),
            name: "Password Generator".into(),
            description: "Generate secure passwords with encrypted history".into(),
            icon: "ShieldCheck".into(),
            category: "Security".into(),
            bundled: true,
            default_enabled: true,
            has_settings: true,
        },
        ExtensionManifest {
            id: "window-management".into(),
            name: "Window Management".into(),
            description: "Snap and arrange windows with keyboard shortcuts".into(),
            icon: "Browsers".into(),
            category: "Utilities".into(),
            bundled: true,
            default_enabled: true,
            has_settings: false,
        },
        ExtensionManifest {
            id: "screenshot".into(),
            name: "Screenshot".into(),
            description: "Capture windows and manage screenshots".into(),
            icon: "Camera".into(),
            category: "Utilities".into(),
            bundled: true,
            default_enabled: true,
            has_settings: false,
        },
        ExtensionManifest {
            id: "ruler".into(),
            name: "Pixel Ruler".into(),
            description: "Measure distances on screen with a pixel ruler overlay".into(),
            icon: "Ruler".into(),
            category: "Utilities".into(),
            bundled: true,
            default_enabled: true,
            has_settings: true,
        },
        ExtensionManifest {
            id: "perf-monitor".into(),
            name: "Performance Monitor".into(),
            description: "Real-time system metrics, charts, and alerts".into(),
            icon: "Gauge".into(),
            category: "Utilities".into(),
            bundled: true,
            default_enabled: true,
            has_settings: true,
        },
    ]
}
