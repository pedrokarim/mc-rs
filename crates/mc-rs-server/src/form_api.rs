//! Form API — port PMMP `src/form/*`. Forms Bedrock (simple, modal, custom)
//! pour afficher des UI dialogs au joueur.

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type")]
pub enum Form {
    #[serde(rename = "form")]
    Simple {
        title: String,
        content: String,
        buttons: Vec<SimpleButton>,
    },
    #[serde(rename = "modal")]
    Modal {
        title: String,
        content: String,
        button1: String,
        button2: String,
    },
    #[serde(rename = "custom_form")]
    Custom {
        title: String,
        content: Vec<CustomFormElement>,
    },
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SimpleButton {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<SimpleButtonImage>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SimpleButtonImage {
    #[serde(rename = "type")]
    pub image_type: String, // "path" or "url"
    pub data: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type")]
pub enum CustomFormElement {
    #[serde(rename = "label")]
    Label { text: String },
    #[serde(rename = "input")]
    Input {
        text: String,
        placeholder: String,
        default: String,
    },
    #[serde(rename = "toggle")]
    Toggle { text: String, default: bool },
    #[serde(rename = "slider")]
    Slider {
        text: String,
        min: f32,
        max: f32,
        step: f32,
        default: f32,
    },
    #[serde(rename = "dropdown")]
    Dropdown {
        text: String,
        options: Vec<String>,
        default: u32,
    },
    #[serde(rename = "step_slider")]
    StepSlider {
        text: String,
        steps: Vec<String>,
        default: u32,
    },
}

impl Form {
    pub fn simple(title: impl Into<String>, content: impl Into<String>) -> Self {
        Self::Simple {
            title: title.into(),
            content: content.into(),
            buttons: Vec::new(),
        }
    }

    pub fn modal(
        title: impl Into<String>,
        content: impl Into<String>,
        button1: impl Into<String>,
        button2: impl Into<String>,
    ) -> Self {
        Self::Modal {
            title: title.into(),
            content: content.into(),
            button1: button1.into(),
            button2: button2.into(),
        }
    }

    pub fn custom(title: impl Into<String>) -> Self {
        Self::Custom {
            title: title.into(),
            content: Vec::new(),
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_form_json() {
        let f = Form::simple("Title", "Hello world");
        let json = f.to_json();
        assert!(json.contains("Title"));
        assert!(json.contains("Hello"));
    }

    #[test]
    fn modal_serializes_buttons() {
        let f = Form::modal("?", "Yes or no?", "Yes", "No");
        let json = f.to_json();
        assert!(json.contains("Yes"));
        assert!(json.contains("No"));
    }
}
