//! `ActionSheet`: the platform's sheet of actions (UIAlertController,
//! a list dialog on Android).

use nsh_core::ui::{Dialog, DialogButton, DialogButtonStyle, prelude::*};

/// A sheet of actions.
#[derive(Debug)]
pub struct ActionSheet;

/// What an action sheet shows.
#[derive(Debug, Clone, Default, PartialEq, JsObject)]
pub struct ActionSheetOptions {
    /// The actions.
    pub options: Vec<SharedString>,
    /// The index of the cancel action.
    pub cancel_button_index: Option<u32>,
    /// The indexes of destructive actions.
    pub destructive_button_indexes: Option<Vec<u32>>,
    /// Above the actions.
    pub title: Option<SharedString>,
    /// Below the title.
    pub message: Option<SharedString>,
}

/// A sheet of actions (UIAlertController action sheet; a list dialog on
/// Android).
#[module]
impl ActionSheet {
    /// Shows the sheet and resolves with the index of the action chosen
    /// (the cancel index, or -1, when it was dismissed).
    pub async fn show(options: ActionSheetOptions) -> i32 {
        let destructive = options
            .destructive_button_indexes
            .clone()
            .unwrap_or_default();
        let index = dialog(Dialog {
            id: 0,
            title: options.title.as_ref().map(|t| t.to_string()),
            message: options.message.as_ref().map(|m| m.to_string()),
            buttons: options
                .options
                .iter()
                .enumerate()
                .map(|(i, text)| DialogButton {
                    text: text.to_string(),
                    style: if Some(i as u32) == options.cancel_button_index {
                        DialogButtonStyle::Cancel
                    } else if destructive.contains(&(i as u32)) {
                        DialogButtonStyle::Destructive
                    } else {
                        DialogButtonStyle::Default
                    },
                })
                .collect(),
            cancelable: true,
            sheet: true,
        })
        .await;
        if index >= 0 {
            index
        } else {
            options.cancel_button_index.map_or(-1, |i| i as i32)
        }
    }
}
