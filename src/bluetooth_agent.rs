use dbus::arg;
use bluetooth_utils;
// use dbus::blocking::Proxy;
use dbus::{Connection, MessageItem, Message};
use dbus::MessageType::Signal;
use bluetooth_session::BluetoothSession;
use dbus::tree::Factory;
use std::{sync::Arc, thread::sleep};
use std::error::Error;
use std::time::Duration;
use std::ptr::null;
use dbus::tree::ObjectPath;
// extern crate dbus;
use dbus::MessageItemArray;
use dbus::Signature;

const AGENT_INTERFACE: &str = "org.bluez.AgentManager1";
const REG_AGENT: &str = "RegisterAgent";
// static SERVICE_NAME: &'static str = "org.bluez";
pub(crate) const MANAGER_PATH: &str = "/org/bluez/agent";

#[derive(Debug)]
pub struct BluetoothAgent<'a> {
    path: String,
    adapter: String,
    conn : Connection,
    session: &'a BluetoothSession,
}
use dbus::{BusType};
use std::sync::{Mutex};

// Define the agent state (registered or not)
struct AgentState {
    registered: bool,
    agent_path: String,
}

impl<'a> BluetoothAgent <'a> {
    fn new(path: String, adapter: String, session: &'a BluetoothSession) -> BluetoothAgent<'a> {
        BluetoothAgent {
            path: path,
            adapter: adapter,
            session: session,
            conn: Connection::get_private(dbus::BusType::System).unwrap(),
        }
    }
    
    pub fn register_agent(session: &'a BluetoothSession) -> Result<(), Box<dyn std::error::Error>> {
      
        
        // Connect to the system bus
        // let conn = Connection::get_private(dbus::BusType::System)?;
        let bluez_path = "/org/bluez";
         let agent_manager_path = "/org/bluez";
        // Define the BlueZ service and the interface
        let bluez_service = "org.bluez";
        let agent_path = "/org/bluez/agent";  // The agent object path
        let object_path = dbus::Path::new(agent_path)?;
        let object_path2 = dbus::Path::new(agent_path)?;
        
        let empty_array: MessageItem = MessageItem::new_array(vec![MessageItem::Str("".into())]).unwrap();
        let empty_array2: MessageItem = MessageItem::new_array(vec![MessageItem::Str("".into())]).unwrap();

        let message_append = vec![
            MessageItem::DictEntry(
                Box::new("org.freedesktop.DBus.Introspectable".into()),
                Box::new(MessageItem::Array(MessageItemArray::new(vec![], dbus::Signature::new("a{sv}").unwrap()).unwrap())),
            ),
            MessageItem::DictEntry(
                Box::new("org.bluez.Agent1".into()),
                Box::new(MessageItem::Array(MessageItemArray::new(vec![], dbus::Signature::new("a{sv}").unwrap()).unwrap())),
            ),
        ];
        let msg = Message::signal(
            &dbus::Path::new("/").unwrap(), // object path
            &dbus::Interface::new("org.freedesktop.DBus.ObjectManager").unwrap(), // interface
            &dbus::Member::new("InterfacesAdded").unwrap(), // member
        )
        .append2(object_path, MessageItem::new_array(message_append).unwrap());
        
        match session.get_connection().send(msg) {
            Ok(_) => println!("Agent registered 1"),
            Err(e) => println!("Error registering agent 1: {:?}", e),
            
        }
        // Create the message for registering the agent
        let msg2 = Message::new_method_call(bluez_service, agent_manager_path, "org.bluez.AgentManager1", "RegisterAgent").
        expect("Failed to create method call")
        .append2(object_path2, "NoInputNoOutput");
    
        // Send the message and block until a reply is received
        // let _response: Message = session.get_connection().send_with_reply_and_block(msg1, 1000)?;
        let _response2: Message = session.get_connection().send_with_reply_and_block(msg2, 1000)?;
        
        // if BluetoothAgent::check_agent_manager_available()? {
        //     println!("AgentManager available");
        // }
        // Notify that the agent has been successfully registered
        println!("Agent registered successfully!");
        // while true {
        //     std::thread::sleep(Duration::from_secs(1));
        // }
        Ok(())
    }

}
