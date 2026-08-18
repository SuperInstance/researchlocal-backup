#![allow(dead_code)]
//! port.rs — Port trait + Telegram adapter
//!
//! Ports are communication channels. The Telegram port receives messages
//! from users and sends responses back.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

use teloxide::prelude::*;
use teloxide::types::ChatId;

// ---------------------------------------------------------------------------
// Port trait
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMessage {
    pub id: String,
    pub text: String,
    pub chat_id: i64,
    pub from_user: Option<String>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortResponse {
    pub text: String,
    pub reply_to: String,
}

#[async_trait]
pub trait Port: Send + Sync {
    /// Receive the next pending message (non-blocking)
    async fn receive(&self) -> Option<PortMessage>;

    /// Send a response
    async fn send(&self, response: &PortResponse) -> Result<(), String>;

    /// Check if the port is active
    fn is_active(&self) -> bool;
}

// ---------------------------------------------------------------------------
// Telegram port
// ---------------------------------------------------------------------------

pub struct TelegramPort {
    bot: Bot,
    incoming: Arc<Mutex<Vec<PortMessage>>>,
    // A simple liveness flag. Kept as an atomic (not a tokio Mutex) so it can be
    // read lock-free from the sync `is_active()` — reaching for `blocking_lock()`
    // inside the async runtime risks a deadlock (and panics on a current-thread
    // runtime).
    active: Arc<AtomicBool>,
}

impl TelegramPort {
    pub fn new(bot_token: &str) -> Self {
        let bot = Bot::new(bot_token);
        Self {
            bot,
            incoming: Arc::new(Mutex::new(Vec::new())),
            active: Arc::new(AtomicBool::new(true)),
        }
    }

    /// Mark the port active/inactive (lock-free, callable from any context).
    pub fn set_active(&self, active: bool) {
        self.active.store(active, Ordering::Relaxed);
    }

    /// Push a message into the incoming queue
    pub async fn push_message(&self, msg: PortMessage) {
        self.incoming.lock().await.push(msg);
    }

    /// Get the underlying bot for setting up the dispatcher
    pub fn bot(&self) -> &Bot {
        &self.bot
    }
}

#[async_trait]
impl Port for TelegramPort {
    async fn receive(&self) -> Option<PortMessage> {
        let mut incoming = self.incoming.lock().await;
        if incoming.is_empty() {
            None
        } else {
            Some(incoming.remove(0))
        }
    }

    async fn send(&self, response: &PortResponse) -> Result<(), String> {
        let chat_id = response
            .reply_to
            .parse::<i64>()
            .map_err(|e| format!("invalid chat_id '{}': {}", response.reply_to, e))?;

        self.bot
            .send_message(ChatId(chat_id), &response.text)
            .await
            .map_err(|e| format!("telegram send error: {}", e))?;

        Ok(())
    }

    fn is_active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }
}

// ---------------------------------------------------------------------------
// Stdio port (for local testing without Telegram)
// ---------------------------------------------------------------------------

pub struct StdioPort {
    incoming: Arc<Mutex<Vec<PortMessage>>>,
}

impl Default for StdioPort {
    fn default() -> Self {
        Self::new()
    }
}

impl StdioPort {
    pub fn new() -> Self {
        Self {
            incoming: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn push_message(&self, msg: PortMessage) {
        self.incoming.lock().await.push(msg);
    }
}

#[async_trait]
impl Port for StdioPort {
    async fn receive(&self) -> Option<PortMessage> {
        self.incoming.lock().await.pop()
    }

    async fn send(&self, response: &PortResponse) -> Result<(), String> {
        println!("[RESPONSE to {}]: {}", response.reply_to, response.text);
        Ok(())
    }

    fn is_active(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn stdio_port_push_and_receive() {
        let port = StdioPort::new();
        let msg = PortMessage { id: "m1".into(), text: "hello".into(), chat_id: 123, from_user: Some("user".into()), timestamp: 0 };
        port.push_message(msg.clone()).await;
        let received = port.receive().await.unwrap();
        assert_eq!(received.text, "hello");
        assert_eq!(received.chat_id, 123);
    }

    #[tokio::test]
    async fn stdio_port_empty_receive() {
        let port = StdioPort::new();
        assert!(port.receive().await.is_none());
    }

    #[tokio::test]
    async fn stdio_port_lifo_order() {
        let port = StdioPort::new();
        for i in 0..3 {
            port.push_message(PortMessage { id: format!("m{}", i), text: format!("msg{}", i), chat_id: 0, from_user: None, timestamp: 0 }).await;
        }
        let first = port.receive().await.unwrap();
        assert_eq!(first.text, "msg2"); // Vec::pop() is LIFO
    }

    #[tokio::test]
    async fn stdio_port_is_always_active() {
        let port = StdioPort::new();
        assert!(port.is_active());
    }

    #[tokio::test]
    async fn stdio_port_send_ok() {
        let port = StdioPort::new();
        let resp = PortResponse { text: "response".into(), reply_to: "chat".into() };
        assert!(port.send(&resp).await.is_ok());
    }

    #[test]
    fn telegram_port_active_flag() {
        let port = TelegramPort::new("dummy-token");
        assert!(port.is_active());
        port.set_active(false);
        assert!(!port.is_active());
        port.set_active(true);
        assert!(port.is_active());
    }

    #[tokio::test]
    async fn telegram_port_push_and_receive() {
        let port = TelegramPort::new("dummy-token");
        let msg = PortMessage { id: "m1".into(), text: "test".into(), chat_id: 42, from_user: None, timestamp: 0 };
        port.push_message(msg).await;
        let received = port.receive().await.unwrap();
        assert_eq!(received.text, "test");
    }

    #[tokio::test]
    async fn telegram_port_empty_receive() {
        let port = TelegramPort::new("dummy-token");
        assert!(port.receive().await.is_none());
    }
}
