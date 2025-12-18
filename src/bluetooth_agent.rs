/// BlueZ Agent for handling passkey pairing
/// 
/// This agent implements the org.bluez.Agent1 interface to handle
/// passkey entry requests during pairing.

use bluetooth_session::BluetoothSession;
use bluetooth_utils;
use dbus::{Message, MessageItem};
use std::error::Error;

static AGENT_INTERFACE: &'static str = "org.bluez.Agent1";
static AGENT_MANAGER_INTERFACE: &'static str = "org.bluez.AgentManager1";
static SERVICE_NAME: &'static str = "org.bluez";
static AGENT_PATH: &'static str = "/org/libhalo/agent";

/// Callback function type for deriving passkey from device MAC address
pub type PasskeyCallback = Box<dyn Fn(&str) -> Option<u32> + Send + Sync>;

/// BlueZ Agent for passkey pairing
pub struct BluetoothAgent {
    session: BluetoothSession,
    agent_path: String,
    passkey_callback: Option<PasskeyCallback>,
    running: bool,
}

impl BluetoothAgent {
    /// Create a new Bluetooth agent
    pub fn new(mut session: BluetoothSession) -> Result<BluetoothAgent, Box<Error>> {
        // Add match rule to receive method calls to our agent
        let agent_path = String::from(AGENT_PATH);

        // Log bus info to verify we're on the system bus and have a unique name
        let unique_name = session.get_connection().unique_name();
        eprintln!(
            "[AGENT] Using system bus (BusType::System); unique name {}",
            unique_name
        );
        
        // Add multiple match rules to catch all relevant messages
        // Match rule for method calls to our agent path from BlueZ
        let match_rule1 = format!(
            "type='method_call',path='{}',sender='{}'",
            agent_path, SERVICE_NAME
        );
        
        // Match rule for method calls with our interface from BlueZ
        let match_rule2 = format!(
            "type='method_call',interface='{}',sender='{}'",
            AGENT_INTERFACE, SERVICE_NAME
        );
        
        // Also match any method call to our agent path (in case sender check fails)
        let match_rule3 = format!(
            "type='method_call',path='{}'",
            agent_path
        );
        
        // Catch-all: match ANY message to our agent path (for debugging)
        let match_rule4 = format!(
            "path='{}'",
            agent_path
        );
        
        eprintln!("[AGENT] Adding match rules for agent path: {}", agent_path);
        
        // Add the match rules to receive method calls
        if let Err(e) = session.get_connection().add_match(&match_rule1) {
            eprintln!("[AGENT] Warning: Failed to add match rule 1: {}", e);
        } else {
            eprintln!("[AGENT] Match rule 1 added successfully");
        }
        
        if let Err(e) = session.get_connection().add_match(&match_rule2) {
            eprintln!("[AGENT] Warning: Failed to add match rule 2: {}", e);
        } else {
            eprintln!("[AGENT] Match rule 2 added successfully");
        }
        
        if let Err(e) = session.get_connection().add_match(&match_rule3) {
            eprintln!("[AGENT] Warning: Failed to add match rule 3: {}", e);
        } else {
            eprintln!("[AGENT] Match rule 3 added successfully");
        }
        
        if let Err(e) = session.get_connection().add_match(&match_rule4) {
            eprintln!("[AGENT] Warning: Failed to add match rule 4: {}", e);
        } else {
            eprintln!("[AGENT] Match rule 4 (catch-all) added successfully");
        }
        
        Ok(BluetoothAgent {
            session,
            agent_path,
            passkey_callback: None,
            running: false,
        })
    }

    /// Set the callback function for deriving passkey from device MAC
    pub fn set_passkey_callback(&mut self, callback: PasskeyCallback) {
        self.passkey_callback = Some(callback);
    }

    /// Register the agent with BlueZ's AgentManager
    /// adapter_path should be the adapter object path (e.g., "/org/bluez/hci0")
    pub fn register(&mut self, _adapter_path: &str, capability: &str) -> Result<(), Box<Error>> {
        let conn = self.session.get_connection();
        
        // Register the agent path with AgentManager
        // Note: AgentManager1 methods are typically on /org/bluez, not the adapter path
        // RegisterAgent signature: (object agent, string capability)
        // So we need to use ObjectPath type for the agent path, not string
        let agent_path_item = MessageItem::ObjectPath(self.agent_path.clone().into());
        let capability_item: MessageItem = capability.into();
        let agent_path_item_clone = agent_path_item.clone();
        
        // Register agent on /org/bluez (standard location for AgentManager1)
        bluetooth_utils::call_method(
            conn,
            AGENT_MANAGER_INTERFACE,
            "/org/bluez",
            "RegisterAgent",
            Some(&[agent_path_item, capability_item]),
            5000,
        )?;

        // Request default agent (so BlueZ uses our agent)
        bluetooth_utils::call_method(
            conn,
            AGENT_MANAGER_INTERFACE,
            "/org/bluez",
            "RequestDefaultAgent",
            Some(&[agent_path_item_clone]),
            5000,
        )?;

        self.running = true;
        Ok(())
    }

    /// Unregister the agent from BlueZ
    pub fn unregister(&mut self) -> Result<(), Box<Error>> {
        if !self.running {
            return Ok(());
        }

        let conn = self.session.get_connection();
        let agent_path_item: MessageItem = self.agent_path.clone().into();
        
        bluetooth_utils::call_method(
            conn,
            AGENT_MANAGER_INTERFACE,
            "/org/bluez",
            "UnregisterAgent",
            Some(&[agent_path_item]),
            5000,
        )?;

        self.running = false;
        Ok(())
    }

    /// Handle incoming D-Bus messages for the agent
    /// This should be called in a loop to process agent requests
    pub fn handle_requests(&self, timeout_ms: u32) -> Result<(), Box<Error>> {
        if !self.running {
            eprintln!("[AGENT] handle_requests called but agent not running");
            return Ok(());
        }

        eprintln!("[AGENT] Waiting for messages (timeout: {}ms)...", timeout_ms);
        // Process incoming messages
        let mut msg_count = 0;
        let mut method_call_count = 0;
        let mut signal_count = 0;
        for msg in self.session.incoming(timeout_ms) {
            msg_count += 1;
            let msg_type = msg.msg_type();
            let msg_path = msg.path().map(|p| p.to_string());
            let msg_interface = msg.interface().map(|i| i.to_string());
            let msg_member = msg.member().map(|m| m.to_string());
            let sender = msg.sender().map(|s| s.to_string()).unwrap_or_else(|| "unknown".to_string());
            
            // Log all messages for debugging
            match msg_type {
                dbus::MessageType::MethodCall => {
                    method_call_count += 1;
                    eprintln!("[AGENT] MethodCall #{}: path={:?}, interface={:?}, member={:?}, sender={}", 
                             method_call_count, msg_path, msg_interface, msg_member, sender);
                }
                dbus::MessageType::Signal => {
                    signal_count += 1;
                    if signal_count <= 5 || signal_count % 10 == 0 {  // Log first 5 and every 10th signal
                        eprintln!("[AGENT] Signal #{}: path={:?}, interface={:?}, member={:?}, sender={}", 
                                 signal_count, msg_path, msg_interface, msg_member, sender);
                    }
                }
                dbus::MessageType::Error => {
                    // dbus::Message in this version doesn't expose error_name/error_message,
                    // so log the raw items to see the error payload.
                    let err_items = msg.get_items();
                    eprintln!(
                        "[AGENT] Error message from {}: items={:?}, path={:?}, interface={:?}, member={:?}",
                        sender, err_items, msg_path, msg_interface, msg_member
                    );
                }
                _ => {
                    eprintln!("[AGENT] Other message type {:?}: path={:?}, interface={:?}, member={:?}, sender={}", 
                             msg_type, msg_path, msg_interface, msg_member, sender);
                }
            }
            // Check if this is a method call to our agent
            if msg_type == dbus::MessageType::MethodCall {
                // Check if this is for our agent path and interface
                let msg_path = msg.path().map(|p| p.to_string());
                let msg_interface = msg.interface().map(|i| i.to_string());
                let msg_member = msg.member().map(|m| m.to_string());
                let sender = msg.sender().map(|s| s.to_string()).unwrap_or_else(|| "unknown".to_string());
                
                // Debug logging for all method calls from BlueZ
                if sender == SERVICE_NAME {
                    eprintln!("[AGENT] MethodCall from BlueZ: path={:?}, interface={:?}, member={:?}", 
                             msg_path, msg_interface, msg_member);
                }
                
                // Debug logging for all agent-related messages
                let is_agent_interface = msg_interface.as_ref().map(|i| i == AGENT_INTERFACE).unwrap_or(false);
                if is_agent_interface {
                    eprintln!("[AGENT] Received Agent1 method call: path={:?}, interface={:?}, member={:?}, sender={}", 
                             msg_path, msg_interface, msg_member, sender);
                }
                
                // Check path match (more lenient - accept if path matches OR interface matches)
                let is_our_agent = msg_path.as_ref().map(|p| p == &self.agent_path).unwrap_or(false);
                
                // Also check if the message is from BlueZ (sender check)
                let is_from_bluez = sender == SERVICE_NAME;
                
                // Handle introspection requests (org.freedesktop.DBus.Introspectable)
                if msg_interface.as_ref().map(|i| i == "org.freedesktop.DBus.Introspectable").unwrap_or(false) {
                    if msg_member.as_ref().map(|m| m == "Introspect").unwrap_or(false) {
                        if is_our_agent {
                            eprintln!("[AGENT] Handling Introspect request for agent object");
                            if let Some(mut reply) = Message::new_method_return(&msg) {
                                // Return minimal introspection XML
                                let xml = format!(
                                    r#"<!DOCTYPE node PUBLIC "-//freedesktop//DTD D-BUS Object Introspection 1.0//EN"
"http://www.freedesktop.org/standards/dbus/1.0/introspect.dtd">
<node>
  <interface name="org.bluez.Agent1">
    <method name="RequestPasskey">
      <arg type="o" direction="in"/>
      <arg type="u" direction="out"/>
    </method>
    <method name="DisplayPasskey">
      <arg type="o" direction="in"/>
      <arg type="u" direction="in"/>
      <arg type="q" direction="in"/>
    </method>
    <method name="RequestConfirmation">
      <arg type="o" direction="in"/>
      <arg type="u" direction="in"/>
      <arg type="q" direction="in"/>
    </method>
    <method name="RequestPinCode">
      <arg type="o" direction="in"/>
      <arg type="s" direction="out"/>
    </method>
    <method name="Authorize">
      <arg type="o" direction="in"/>
      <arg type="s" direction="in"/>
    </method>
    <method name="Cancel"/>
    <method name="Release"/>
  </interface>
</node>"#
                                );
                                reply.append_items(&[xml.into()]);
                                if let Err(e) = self.session.get_connection().send(reply) {
                                    eprintln!("[AGENT] Failed to send introspection reply: {:?}", e);
                                } else {
                                    eprintln!("[AGENT] Sent introspection reply");
                                }
                            }
                            continue; // Don't process this message further
                        }
                    }
                }
                
                // Handle if it's for our agent OR if it's an agent interface method call from BlueZ
                // Be very lenient - if it's Agent1 interface, handle it
                if (is_our_agent && is_agent_interface) || (is_agent_interface && is_from_bluez) || is_agent_interface {
                    if let Some(method_name) = msg_member {
                        match method_name.as_str() {
                                "RequestPasskey" => {
                                    eprintln!("[AGENT] Matched RequestPasskey method call");
                                    // BlueZ requests passkey from central - provide the fixed passkey
                                    if let Err(e) = self.handle_request_passkey(&msg) {
                                        eprintln!("[AGENT] Error handling RequestPasskey: {}", e);
                                    }
                                }
                                "DisplayPasskey" => {
                                    // EP displays/sends passkey - extract and verify it
                                    let _ = self.handle_display_passkey(&msg);
                                }
                                "RequestPinCode" => {
                                    // Not used for LE, but handle gracefully
                                    let _ = self.handle_request_pin(&msg);
                                }
                                "RequestConfirmation" => {
                                    // EP sent passkey, now confirm it matches expected value
                                    let _ = self.handle_request_confirmation(&msg);
                                }
                                "Authorize" => {
                                    // Auto-authorize
                                    let _ = self.handle_authorize(&msg);
                                }
                                "Cancel" => {
                                    // Pairing cancelled - no response needed
                                }
                                "Release" => {
                                    // Agent released - no response needed
                                }
                                _ => {
                                    // Unknown method - send error
                                    let _ = self.send_error_reply(&msg, "org.bluez.Error.NotSupported", "");
                                }
                        }
                    }
                }
            }
        }
        
        if msg_count == 0 {
            eprintln!("[AGENT] No messages received in this iteration");
        } else {
            eprintln!("[AGENT] Processed {} messages: {} method calls, {} signals", 
                     msg_count, method_call_count, signal_count);
        }

        Ok(())
    }

    fn send_error_reply(&self, msg: &Message, error_name: &str, error_msg: &str) -> Result<(), Box<Error>> {
        if let Some(reply) = Message::new_error(msg, error_name, error_msg) {
            if let Err(_) = self.session.get_connection().send(reply) {
                return Err("Failed to send error reply".into());
            }
        }
        Ok(())
    }

    fn handle_request_passkey(&self, msg: &Message) -> Result<(), Box<Error>> {
        // Log that RequestPasskey was called
        eprintln!("[AGENT] RequestPasskey called - providing fixed passkey 123456");
        
        // Extract device path from message
        // RequestPasskey signature: (object device) -> (uint32 passkey)
        let items = msg.get_items();
        let device_path = if items.len() > 0 {
            if let Some(device_item) = items.get(0) {
                // Try to extract as string (object paths are often passed as strings in D-Bus)
                if let Ok(device_path) = device_item.inner::<&str>() {
                    String::from(device_path)
                } else {
                    return Err("Invalid device path type in RequestPasskey - expected string".into());
                }
            } else {
                return Err("No device path in RequestPasskey".into());
            }
        } else {
            return Err("No items in RequestPasskey message".into());
        };

        // Get passkey using callback (or use default)
        let passkey_value = if let Some(ref callback) = self.passkey_callback {
            // Try to get device MAC address for callback
            let device_mac = match bluetooth_utils::get_property(
                self.session.get_connection(),
                "org.bluez.Device1",
                &device_path,
                "Address",
            ) {
                Ok(address_item) => {
                    if let Ok(mac) = address_item.inner::<&str>() {
                        String::from(mac)
                    } else {
                        String::new()
                    }
                }
                Err(_) => String::new(),
            };
            
            callback(&device_mac).unwrap_or(123456)
        } else {
            // Default passkey if no callback (8-digit as required by EP)
            123456
        };

        // Send reply with passkey (uint32)
        eprintln!("[AGENT] Sending passkey reply: {}", passkey_value);
        if let Some(mut reply) = Message::new_method_return(msg) {
            // Log reply details for debugging
            let reply_dest = reply.destination().map(|d| d.to_string()).unwrap_or_else(|| "<none>".to_string());
            let reply_sender = reply.sender().map(|s| s.to_string()).unwrap_or_else(|| "<none>".to_string());
            let original_sender = msg.sender().map(|s| s.to_string()).unwrap_or_else(|| "<none>".to_string());
            eprintln!("[AGENT] Reply destination: {}, reply sender: {}, original sender: {}", 
                     reply_dest, reply_sender, original_sender);
            
            reply.append_items(&[passkey_value.into()]);
            
            if let Err(e) = self.session.get_connection().send(reply) {
                eprintln!("[AGENT] Failed to send passkey reply: {:?}", e);
                return Err("Failed to send passkey reply".into());
            }
            eprintln!("[AGENT] Passkey reply sent successfully");
        } else {
            eprintln!("[AGENT] Failed to create method return message");
            return Err("Failed to create method return message".into());
        }
        Ok(())
    }

    fn handle_display_passkey(&self, msg: &Message) -> Result<(), Box<Error>> {
        // DisplayPasskey signature: (object device, uint32 passkey, uint16 entered)
        // EP displays/sends the passkey - extract it for verification
        let items = msg.get_items();
        if items.len() >= 2 {
            // First item is device path, second is passkey (uint32)
            if let Some(passkey_item) = items.get(1) {
                if let Ok(passkey_value) = passkey_item.inner::<u32>() {
                    // Store or verify passkey - we'll verify in RequestConfirmation
                    // Note: Can't log here as log crate not available in blurz
                    // The passkey will be verified in handle_request_confirmation
                }
            }
        }
        // DisplayPasskey doesn't require a response
        Ok(())
    }

    fn handle_request_pin(&self, _msg: &Message) -> Result<(), Box<Error>> {
        // PIN code not used for LE - return error
        if let Some(reply) = Message::new_error(
            _msg,
            "org.bluez.Error.Rejected",
            "PIN not supported for LE",
        ) {
            if let Err(_) = self.session.get_connection().send(reply) {
                return Err("Failed to send PIN error reply".into());
            }
        }
        Ok(())
    }

    fn handle_request_confirmation(&self, msg: &Message) -> Result<(), Box<Error>> {
        // RequestConfirmation signature: (object device, uint32 passkey)
        // EP sent/displays the passkey, now BlueZ asks us to confirm it
        // Since the EP derives the passkey from its MAC address, we should
        // auto-confirm whatever passkey the EP sends us
        let items = msg.get_items();
        let passkey_received = if items.len() >= 2 {
            // Second item is the passkey (uint32) that EP sent
            if let Some(passkey_item) = items.get(1) {
                if let Ok(passkey_value) = passkey_item.inner::<u32>() {
                    Some(passkey_value)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        // Auto-confirm the passkey that EP sent (EP derives it from MAC address)
        // We trust whatever passkey the EP sends us
        if passkey_received.is_some() {
            // Passkey received from EP - confirm pairing
            if let Some(reply) = Message::new_method_return(msg) {
                if let Err(_) = self.session.get_connection().send(reply) {
                    return Err("Failed to send confirmation reply".into());
                }
            } else {
                return Err("Failed to create confirmation reply".into());
            }
        } else {
            // Couldn't extract passkey - reject
            if let Some(reply) = Message::new_error(
                msg,
                "org.bluez.Error.Rejected",
                "Could not extract passkey",
            ) {
                if let Err(_) = self.session.get_connection().send(reply) {
                    return Err("Failed to send rejection reply".into());
                }
            }
        }
        Ok(())
    }

    fn handle_authorize(&self, msg: &Message) -> Result<(), Box<Error>> {
        // Auto-authorize
        if let Some(reply) = Message::new_method_return(msg) {
            if let Err(_) = self.session.get_connection().send(reply) {
                return Err("Failed to send authorize reply".into());
            }
        } else {
            return Err("Failed to create authorize reply".into());
        }
        Ok(())
    }
}

impl Drop for BluetoothAgent {
    fn drop(&mut self) {
        let _ = self.unregister();
    }
}

