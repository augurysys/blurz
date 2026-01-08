use bluetooth_session::BluetoothSession;
use dbus::{BusType, Connection, Message, MessageItem, MessageType, Path};
use std::error::Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

const SERVICE_NAME: &str = "org.bluez";
const AGENT_MANAGER_PATH: &str = "/org/bluez";
const AGENT_MANAGER_INTERFACE: &str = "org.bluez.AgentManager1";
const AGENT_INTERFACE: &str = "org.bluez.Agent1";
const AGENT_PATH: &str = "/org/bluez/agent";

/// Capability for the authentication agent
#[derive(Clone, Debug)]
pub enum AgentCapability {
    DisplayOnly,
    DisplayYesNo,
    KeyboardOnly,
    NoInputNoOutput,
    KeyboardDisplay,
    Empty, // Empty string capability (matches bluetoothctl default)
}

impl AgentCapability {
    fn as_str(&self) -> &str {
        match self {
            AgentCapability::DisplayOnly => "DisplayOnly",
            AgentCapability::DisplayYesNo => "DisplayYesNo",
            AgentCapability::KeyboardOnly => "KeyboardOnly",
            AgentCapability::NoInputNoOutput => "NoInputNoOutput",
            AgentCapability::KeyboardDisplay => "KeyboardDisplay",
            AgentCapability::Empty => "", // Empty string matches bluetoothctl
        }
    }
}

/// Bluetooth authentication agent for handling pairing and authorization
pub struct BluetoothAuthAgent {
    session: BluetoothSession,
    capability: AgentCapability,
    object_path: String,
    passkey: u32,
    running: Arc<AtomicBool>,
}

impl BluetoothAuthAgent {
    /// Create a new authentication agent with a pre-calculated passkey
    pub fn new_with_passkey(
        session: BluetoothSession,
        capability: AgentCapability,
        passkey: u32,
    ) -> Result<BluetoothAuthAgent, Box<Error>> {
        Ok(BluetoothAuthAgent {
            session,
            capability,
            object_path: AGENT_PATH.to_string(),
            passkey,
            running: Arc::new(AtomicBool::new(true)),
        })
    }

    pub fn new(
        session: BluetoothSession,
        capability: AgentCapability,
    ) -> Result<BluetoothAuthAgent, Box<Error>> {
        Ok(BluetoothAuthAgent {
            session,
            capability,
            object_path: AGENT_PATH.to_string(),
            passkey: 0,
            running: Arc::new(AtomicBool::new(true)),
        })
    }

    /// Register the agent with BlueZ
    pub fn register(&self) -> Result<(), Box<Error>> {
        let conn = self.session.get_connection();
        let mut m = Message::new_method_call(
            SERVICE_NAME,
            AGENT_MANAGER_PATH,
            AGENT_MANAGER_INTERFACE,
            "RegisterAgent",
        )?;
        
        // Parameters: object_path (o), capability (s)
        m.append_items(&[
            MessageItem::ObjectPath(Path::new(AGENT_PATH)?),
            MessageItem::Str(self.capability.as_str().to_string()),
        ]);
        
        conn.send_with_reply_and_block(m, 1000)?;
        Ok(())
    }

    /// Request to be the default agent
    pub fn request_default(&self) -> Result<(), Box<Error>> {
        let conn = self.session.get_connection();
        let mut m = Message::new_method_call(
            SERVICE_NAME,
            AGENT_MANAGER_PATH,
            AGENT_MANAGER_INTERFACE,
            "RequestDefaultAgent",
        )?;
        
        // Parameters: object_path (o)
        m.append_items(&[MessageItem::ObjectPath(Path::new(AGENT_PATH)?)]);
        
        conn.send_with_reply_and_block(m, 1000)?;
        Ok(())
    }

    /// Unregister the agent
    pub fn unregister(&self) -> Result<(), Box<Error>> {
        let conn = self.session.get_connection();
        let mut m = Message::new_method_call(
            SERVICE_NAME,
            AGENT_MANAGER_PATH,
            AGENT_MANAGER_INTERFACE,
            "UnregisterAgent",
        )?;
        
        // Parameters: object_path (o)
        m.append_items(&[MessageItem::ObjectPath(Path::new(AGENT_PATH)?)]);
        
        conn.send_with_reply_and_block(m, 1000)?;
        Ok(())
    }

    /// Stop the agent
    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }

    /// Handle incoming D-Bus messages
    pub fn handle_messages(&self, timeout_ms: i32) {
        let conn = self.session.get_connection();
        let object_path = self.object_path.clone();
        let passkey = self.passkey;
        
        let mut method_call_count = 0;
        let mut signal_count = 0;
        
        // Receive messages with timeout - incoming() returns an iterator
        for msg in conn.incoming(timeout_ms) {
            let msg_type = msg.msg_type();
            let msg_path = msg.path();
            let msg_interface = msg.interface();
            let msg_member = msg.member();
            let msg_sender = msg.sender().map(|s| format!("{}", s)).unwrap_or_else(|| "no sender".to_string());
            let msg_destination = msg.destination().map(|d| format!("{}", d)).unwrap_or_else(|| "no destination".to_string());
            
            let msg_path_str = msg_path.map(|p| format!("{}", p)).unwrap_or_else(|| "no path".to_string());
            let msg_interface_str = msg_interface.map(|i| format!("{}", i)).unwrap_or_else(|| "no interface".to_string());
            let msg_member_str = msg_member.map(|m| format!("{}", m)).unwrap_or_else(|| "no member".to_string());
            
            match msg_type {
                MessageType::MethodCall => {
                    method_call_count += 1;
                    eprintln!("[auth_agent] METHOD CALL: {} on {} at path {} from {} to {}", 
                        msg_member_str, msg_interface_str, msg_path_str, msg_sender, msg_destination);
                    
                    if msg_path_str == *object_path {
                        eprintln!("[auth_agent] ✓ This is for our agent! Handling method: {}", msg_member_str);
                        if let Some(reply) = handle_agent_message(&msg, object_path.clone(), passkey) {
                            // Log reply details before sending
                            let reply_dest = reply.destination().map(|d| format!("{}", d)).unwrap_or_else(|| "no destination".to_string());
                            let reply_type = format!("{:?}", reply.msg_type());
                            eprintln!("[auth_agent] Sending reply: type={}, destination={}", reply_type, reply_dest);
                            
                            // Send the reply
                            // NOTE: D-Bus may reject method returns with "0 matched rules" error
                            // This is a D-Bus security policy issue, not a code issue
                            // The reply is correctly formatted, but D-Bus security is blocking it
                            // This might require D-Bus policy configuration or a different approach
                            let _serial = conn.send(reply);
                            eprintln!("[auth_agent] Reply sent (serial: {:?}) - Note: D-Bus may reject with '0 matched rules' error", _serial);
                        } else {
                            eprintln!("[auth_agent] No reply generated for method: {}", msg_member_str);
                        }
                    } else if msg_interface_str == "org.bluez.Agent1" {
                        eprintln!("[auth_agent] ⚠ Agent1 method call but wrong path: {} (expected: {})", msg_path_str, object_path);
                    }
                }
                MessageType::MethodReturn => {
                    eprintln!("[auth_agent] METHOD RETURN: from {} to {}", msg_sender, msg_destination);
                }
                MessageType::Error => {
                    // Extract error information from message items
                    let items = msg.get_items();
                    let error_info = if !items.is_empty() {
                        format!("items: {:?}", items)
                    } else {
                        "no items".to_string()
                    };
                    eprintln!("[auth_agent] D-Bus ERROR: {} - {} (from {} to {})", 
                        msg_interface_str, error_info, msg_sender, msg_destination);
                }
                MessageType::Signal => {
                    signal_count += 1;
                    // Only log non-BlueZ signals to reduce noise
                    if !msg_sender.contains("org.bluez") && !msg_sender.starts_with(":1.") {
                        eprintln!("[auth_agent] Signal (non-BlueZ): {} on {} at path {} from {}", 
                            msg_member_str, msg_interface_str, msg_path_str, msg_sender);
                    } else {
                        eprintln!("[auth_agent] Signal: {} on {} at path {} from {}", 
                            msg_member_str, msg_interface_str, msg_path_str, msg_sender);
                    }
                }
                _ => {
                    eprintln!("[auth_agent] Unknown message type: {:?}", msg_type);
                }
            }
        }
        
        if method_call_count > 0 || signal_count > 0 {
            eprintln!("[auth_agent] Processed {} message(s) this iteration ({} method calls, {} signals)", 
                method_call_count + signal_count, method_call_count, signal_count);
        }
    }
}

/// Handle agent method calls
fn handle_agent_message(msg: &Message, object_path: String, passkey: u32) -> Option<Message> {
    let method = msg.member().map(|m| format!("{}", m)).unwrap_or_else(|| "unknown".to_string());
    let method_str = method.as_str();
    
    match method_str {
        "RequestPasskey" => {
            eprintln!("[auth_agent] Agent received method call: RequestPasskey from {} (path: {})", 
                msg.sender().map(|s| format!("{}", s)).unwrap_or_else(|| "no sender".to_string()),
                msg.path().map(|p| format!("{}", p)).unwrap_or_else(|| "no path".to_string()));
            
            if let Some(device_path) = extract_device_path(msg) {
                eprintln!("[auth_agent] Agent RequestPasskey() called for device: {}", device_path);
                eprintln!("[auth_agent] Returning passkey {}", passkey);
                
                // Return passkey as u32
                create_reply(msg, &[MessageItem::UInt32(passkey)])
            } else {
                eprintln!("[auth_agent] ERROR: RequestPasskey called but couldn't extract device path");
                create_error_reply(msg, "org.bluez.Error.InvalidArguments", "Invalid device path")
            }
        }
        "DisplayPasskey" => {
            if let Some((device_path, passkey_val, entered)) = extract_display_passkey_info(msg) {
                eprintln!("[auth_agent] Agent DisplayPasskey() called: device={}, passkey={:06}, entered={}", 
                    device_path, passkey_val, entered);
            }
            create_reply(msg, &[])
        }
        "RequestConfirmation" => {
            if let Some((device_path, passkey_val)) = extract_confirmation_info(msg) {
                eprintln!("[auth_agent] Agent RequestConfirmation() called: device={}, passkey={:06}", 
                    device_path, passkey_val);
                eprintln!("[auth_agent] Auto-confirming passkey");
            }
            create_reply(msg, &[])
        }
        "RequestAuthorization" => {
            if let Some(device_path) = extract_device_path(msg) {
                eprintln!("[auth_agent] Agent RequestAuthorization() called for device: {}", device_path);
                eprintln!("[auth_agent] Auto-authorizing");
            }
            create_reply(msg, &[])
        }
        "AuthorizeService" => {
            if let Some((device_path, uuid)) = extract_service_info(msg) {
                eprintln!("[auth_agent] Agent AuthorizeService() called: device={}, uuid={}", device_path, uuid);
                eprintln!("[auth_agent] Auto-authorizing service");
            }
            create_reply(msg, &[])
        }
        "Release" => {
            eprintln!("[auth_agent] Agent Release() called");
            create_reply(msg, &[])
        }
        "Cancel" => {
            eprintln!("[auth_agent] Agent Cancel() called");
            create_reply(msg, &[])
        }
        _ => {
            println!("[auth_agent] Agent received unknown method: {}", method_str);
            create_error_reply(
                msg,
                "org.bluez.Error.NotSupported",
                "Method not supported",
            )
        }
    }
}

/// Extract device path from agent method call message
fn extract_device_path(msg: &Message) -> Option<String> {
    let items = msg.get_items();
    if let Some(MessageItem::Str(path)) = items.get(0) {
        return Some(path.clone());
    }
    None
}

/// Extract device path and UUID from AuthorizeService message
fn extract_service_info(msg: &Message) -> Option<(String, String)> {
    let items = msg.get_items();
    if items.len() >= 2 {
        if let (Some(MessageItem::Str(device_path)), Some(MessageItem::Str(uuid))) =
            (items.get(0), items.get(1))
        {
            return Some((device_path.clone(), uuid.clone()));
        }
    }
    None
}

/// Extract device path, passkey, and entered count from DisplayPasskey message
fn extract_display_passkey_info(msg: &Message) -> Option<(String, u32, u16)> {
    let items = msg.get_items();
    if items.len() >= 3 {
        if let (Some(MessageItem::Str(device_path)), Some(MessageItem::UInt32(passkey)), Some(MessageItem::UInt16(entered))) =
            (items.get(0), items.get(1), items.get(2))
        {
            return Some((device_path.clone(), *passkey, *entered));
        }
    }
    None
}

/// Extract device path and passkey from RequestConfirmation message
fn extract_confirmation_info(msg: &Message) -> Option<(String, u32)> {
    let items = msg.get_items();
    if items.len() >= 2 {
        if let (Some(MessageItem::Str(device_path)), Some(MessageItem::UInt32(passkey))) =
            (items.get(0), items.get(1))
        {
            return Some((device_path.clone(), *passkey));
        }
    }
    None
}

/// Create a reply message
fn create_reply(original: &Message, items: &[MessageItem]) -> Option<Message> {
    // Log original message details for debugging
    let orig_serial = original.get_serial();
    let orig_dest = original.destination().map(|d| format!("{}", d)).unwrap_or_else(|| "no destination".to_string());
    let orig_sender = original.sender().map(|s| format!("{}", s)).unwrap_or_else(|| "no sender".to_string());
    let orig_path = original.path().map(|p| format!("{}", p)).unwrap_or_else(|| "no path".to_string());
    let orig_interface = original.interface().map(|i| format!("{}", i)).unwrap_or_else(|| "no interface".to_string());
    let orig_member = original.member().map(|m| format!("{}", m)).unwrap_or_else(|| "no member".to_string());
    eprintln!("[auth_agent] Original call: serial={}, destination={}, sender={}, path={}, interface={}, member={}", 
        orig_serial, orig_dest, orig_sender, orig_path, orig_interface, orig_member);
    
    // Create method return - this should set reply serial and destination automatically
    if let Some(mut reply) = Message::new_method_return(original) {
        // Extract original interface and member for logging
        // Note: Method returns in D-Bus are matched by serial number, not by interface/member
        // However, D-Bus security policies may check these fields, so we log them
        let reply_dest = reply.destination().map(|d| format!("{}", d)).unwrap_or_else(|| "no destination".to_string());
        let reply_path = reply.path().map(|p| format!("{}", p)).unwrap_or_else(|| "no path".to_string());
        let reply_interface = reply.interface().map(|i| format!("{}", i)).unwrap_or_else(|| "no interface".to_string());
        let reply_member = reply.member().map(|m| format!("{}", m)).unwrap_or_else(|| "no member".to_string());
        eprintln!("[auth_agent] Created reply: destination={}, path={}, interface={}, member={}, items={}, original_serial={}", 
            reply_dest, reply_path, reply_interface, reply_member, items.len(), orig_serial);
        eprintln!("[auth_agent] Original had: path={}, interface={}, member={}", orig_path, orig_interface, orig_member);
        
        // Add reply items
        if !items.is_empty() {
            reply.append_items(items);
        }
        
        // Note: The dbus crate 0.6.4's Message::new_method_return() doesn't preserve
        // interface/member in the reply. Method returns are matched by serial number,
        // but D-Bus security policies may still check these fields.
        // If the policy check fails, we'll see "0 matched rules" error.
        Some(reply)
    } else {
        eprintln!("[auth_agent] ERROR: Failed to create method return reply");
        None
    }
}

/// Create an error reply message
fn create_error_reply(original: &Message, error_name: &str, error_msg: &str) -> Option<Message> {
    Message::new_error(original, error_name, error_msg)
}

impl Drop for BluetoothAuthAgent {
    fn drop(&mut self) {
        self.stop();
        if let Err(e) = self.unregister() {
            eprintln!("[auth_agent] Failed to unregister agent on drop: {}", e);
        }
    }
}

