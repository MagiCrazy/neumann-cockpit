use crossterm::event::KeyCode;
use tokio::sync::mpsc;

use crate::api::client::ApiClient;
use crate::api::tasks::{fetch_mark_message_read, fetch_send_message};
use crate::app::{ApiMessage, AppState, MessagesInput};

use super::geometry::list_nav;

pub(super) fn handle_messages_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    match &state.messages_input {
        MessagesInput::Browsing { sent_tab, selection } => {
            let sent_tab = *sent_tab;
            let selection = *selection;
            let count = if sent_tab { state.sent_messages.len() } else { state.messages.len() };
            match code {
                KeyCode::Esc | KeyCode::Char('Y') | KeyCode::Char('q') => {
                    state.messages_input = MessagesInput::Inactive;
                }
                KeyCode::Tab | KeyCode::Left | KeyCode::Right | KeyCode::Char('h') | KeyCode::Char('l') => {
                    state.messages_input = MessagesInput::Browsing { sent_tab: !sent_tab, selection: 0 };
                }
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(new_sel) = list_nav(code, selection, count) {
                        state.messages_input = MessagesInput::Browsing { sent_tab, selection: new_sel };
                    }
                }
                KeyCode::Enter => {
                    let id = if sent_tab {
                        state.sent_messages.get(selection).map(|m| m.id)
                    } else {
                        state.messages.get(selection).map(|m| {
                            if m.status == crate::api::types::MessageStatus::Unread {
                                fetch_mark_message_read(m.id, client.clone(), tx.clone());
                            }
                            m.id
                        })
                    };
                    if let Some(id) = id {
                        state.messages_input = MessagesInput::Reading { id, sent_tab };
                    }
                }
                KeyCode::Char('c') => {
                    let recipients = state.collect_message_recipients();
                    if recipients.is_empty() {
                        state.error = Some("no reachable recipient in this sector".into());
                    } else {
                        state.messages_input = MessagesInput::PickRecipient { recipients, selection: 0 };
                    }
                }
                _ => {}
            }
        }
        MessagesInput::Reading { sent_tab, .. } => {
            let sent_tab = *sent_tab;
            if matches!(code, KeyCode::Esc | KeyCode::Char('h') | KeyCode::Left | KeyCode::Char('q')) {
                state.messages_input = MessagesInput::Browsing { sent_tab, selection: 0 };
            }
        }
        MessagesInput::PickRecipient { recipients, selection } => {
            let sel = *selection;
            let count = recipients.len();
            match code {
                KeyCode::Esc => state.messages_input = MessagesInput::Browsing { sent_tab: false, selection: 0 },
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(new_sel) = list_nav(code, sel, count) {
                        if let MessagesInput::PickRecipient { ref mut selection, .. } = state.messages_input {
                            *selection = new_sel;
                        }
                    }
                }
                KeyCode::Enter => {
                    let MessagesInput::PickRecipient { ref recipients, selection } = state.messages_input else { return };
                    let (kind, id, name) = recipients[selection].clone();
                    state.messages_input = MessagesInput::Compose {
                        recipient_type: kind,
                        recipient_id: id,
                        recipient_name: name,
                        body_buf: String::new(),
                        error: None,
                    };
                }
                _ => {}
            }
        }
        MessagesInput::Compose { .. } => match code {
            KeyCode::Esc => state.messages_input = MessagesInput::Browsing { sent_tab: false, selection: 0 },
            KeyCode::Backspace => {
                if let MessagesInput::Compose { ref mut body_buf, .. } = state.messages_input {
                    body_buf.pop();
                }
            }
            KeyCode::Char(c) => {
                if let MessagesInput::Compose { ref mut body_buf, .. } = state.messages_input {
                    body_buf.push(c);
                }
            }
            KeyCode::Enter => {
                let (kind, id, body) = {
                    let MessagesInput::Compose { ref recipient_type, ref recipient_id, ref body_buf, .. } = state.messages_input else { return };
                    if body_buf.trim().is_empty() { return }
                    (recipient_type.clone(), recipient_id.clone(), body_buf.clone())
                };
                fetch_send_message(kind, id, body, client.clone(), tx.clone());
            }
            _ => {}
        },
        MessagesInput::Inactive => {}
    }
}
